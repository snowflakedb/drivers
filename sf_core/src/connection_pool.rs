use snafu::{Location, Snafu};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// Use raw pointer as Handle for now - will integrate with actual handle manager later
pub type Handle = usize;

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub min_size: usize,
    pub max_size: usize,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_size: 1,
            max_size: 10,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600), // 10 minutes
            max_lifetime: Duration::from_secs(3600), // 1 hour
        }
    }
}

struct PooledConnection {
    handle: Handle,
    created_at: Instant,
    last_used: Instant,
}

impl PooledConnection {
    fn new(handle: Handle) -> Self {
        let now = Instant::now();
        Self {
            handle,
            created_at: now,
            last_used: now,
        }
    }

    fn is_expired(&self, config: &PoolConfig) -> bool {
        let now = Instant::now();
        now.duration_since(self.created_at) > config.max_lifetime
    }

    fn is_idle_timeout(&self, config: &PoolConfig) -> bool {
        let now = Instant::now();
        now.duration_since(self.last_used) > config.idle_timeout
    }

    fn touch(&mut self) {
        self.last_used = Instant::now();
    }
}

pub struct ConnectionPool {
    config: PoolConfig,
    available: Arc<Mutex<VecDeque<PooledConnection>>>,
    total_count: Arc<Mutex<usize>>,
}

impl ConnectionPool {
    pub fn new(config: PoolConfig) -> Self {
        Self {
            config,
            available: Arc::new(Mutex::new(VecDeque::new())),
            total_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(PoolConfig::default())
    }

    /// Get a connection from the pool or create a new one
    pub fn get(&self) -> Result<PooledConnectionGuard, ConnectionPoolError> {
        // Try to get from pool
        let mut available = self.available.lock().unwrap();

        // Remove expired connections
        available
            .retain(|conn| !conn.is_expired(&self.config) && !conn.is_idle_timeout(&self.config));

        if let Some(mut conn) = available.pop_front() {
            conn.touch();
            return Ok(PooledConnectionGuard {
                connection: Some(conn),
                pool: self.available.clone(),
            });
        }

        drop(available);

        // Check if we can create a new connection
        let mut total = self.total_count.lock().unwrap();
        if *total >= self.config.max_size {
            return PoolExhaustedSnafu {
                max_size: self.config.max_size,
            }
            .fail();
        }

        // Create new connection (placeholder - would call actual connection creation)
        let handle = 0 as Handle; // TODO: Create actual connection
        *total += 1;
        drop(total);

        Ok(PooledConnectionGuard {
            connection: Some(PooledConnection::new(handle)),
            pool: self.available.clone(),
        })
    }

    /// Return number of available connections
    pub fn available_count(&self) -> usize {
        self.available.lock().unwrap().len()
    }

    /// Return total number of connections
    pub fn total_count(&self) -> usize {
        *self.total_count.lock().unwrap()
    }

    /// Cleanup idle and expired connections
    pub fn cleanup(&self) {
        let mut available = self.available.lock().unwrap();
        let before_count = available.len();
        available
            .retain(|conn| !conn.is_expired(&self.config) && !conn.is_idle_timeout(&self.config));
        let removed = before_count - available.len();

        if removed > 0 {
            let mut total = self.total_count.lock().unwrap();
            *total = total.saturating_sub(removed);
        }
    }
}

pub struct PooledConnectionGuard {
    connection: Option<PooledConnection>,
    pool: Arc<Mutex<VecDeque<PooledConnection>>>,
}

impl PooledConnectionGuard {
    pub fn handle(&self) -> Handle {
        self.connection.as_ref().unwrap().handle
    }
}

impl Drop for PooledConnectionGuard {
    fn drop(&mut self) {
        if let Some(conn) = self.connection.take() {
            // Return connection to pool
            let mut pool = self.pool.lock().unwrap();
            pool.push_back(conn);
        }
    }
}

#[derive(Debug, Snafu)]
pub enum ConnectionPoolError {
    #[snafu(display("Connection pool exhausted (max size: {max_size})"))]
    PoolExhausted {
        max_size: usize,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Connection timeout after {timeout:?}"))]
    ConnectionTimeout {
        timeout: Duration,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Connection validation failed"))]
    ValidationFailed {
        #[snafu(implicit)]
        location: Location,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation() {
        let pool = ConnectionPool::with_defaults();
        assert_eq!(pool.available_count(), 0);
        assert_eq!(pool.total_count(), 0);
    }

    #[test]
    fn test_pool_get_connection() {
        let config = PoolConfig {
            min_size: 1,
            max_size: 5,
            ..Default::default()
        };
        let pool = ConnectionPool::new(config);

        let conn = pool.get().unwrap();
        assert_eq!(pool.total_count(), 1);
        assert_eq!(pool.available_count(), 0);

        drop(conn);
        assert_eq!(pool.available_count(), 1);
    }

    #[test]
    fn test_pool_max_size() {
        let config = PoolConfig {
            min_size: 1,
            max_size: 2,
            ..Default::default()
        };
        let pool = ConnectionPool::new(config);

        let _conn1 = pool.get().unwrap();
        let _conn2 = pool.get().unwrap();

        // Should fail - pool exhausted
        let result = pool.get();
        assert!(result.is_err());
    }
}
