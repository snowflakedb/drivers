use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::model::{Direction, HandleType};

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum SplitMode {
    Env,
    Connection,
    Statement,
}

pub struct SplitStats {
    pub files_written: usize,
    pub blocks_processed: usize,
    pub envs_found: usize,
    pub connections_found: usize,
    pub statements_found: usize,
}

pub fn split_trace(
    input: &Path,
    output_dir: &Path,
    mode: SplitMode,
    require_statements: bool,
) -> io::Result<SplitStats> {
    let content = fs::read_to_string(input)?;
    let mut raw_blocks = extract_raw_blocks(&content);
    let hierarchy = build_hierarchy_and_assign_scopes(&mut raw_blocks);
    write_split_output(
        &raw_blocks,
        &hierarchy,
        output_dir,
        mode,
        require_statements,
    )
}

// -- Raw block extraction --

#[derive(Debug, Clone)]
enum HandleScope {
    Env(String),
    Connection(String),
    Statement(String),
    Unknown,
}

#[derive(Debug)]
struct RawBlock {
    raw_text: String,
    thread_id: String,
    function_name: String,
    direction: Direction,
    scope: HandleScope,
    env_addr: Option<String>,
    conn_addr: Option<String>,
    stmt_addr: Option<String>,
    handle_type: Option<i64>,
    input_handle: Option<String>,
    output_handle: Option<String>,
}

static HEADER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[ODBC\]\[(\d+)\]\[([^\]]+)\]\[([^\]]+)\]\[(\d+)\]").unwrap());

static EXIT_DIR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"Exit:\[(\w+)\]").unwrap());

fn extract_raw_blocks(content: &str) -> Vec<RawBlock> {
    let header_re = &*HEADER_RE;
    let mut blocks = Vec::new();
    let mut cur_raw = String::new();
    let mut cur_meta: Option<(String, String)> = None;
    let mut cur_body: Vec<String> = Vec::new();

    for line in content.lines() {
        if let Some(caps) = header_re.captures(line) {
            if let Some((thread_id, source_file)) = cur_meta.take() {
                blocks.push(finalize_block(
                    std::mem::take(&mut cur_raw),
                    thread_id,
                    source_file,
                    std::mem::take(&mut cur_body),
                ));
            }
            cur_raw.push_str(line);
            cur_raw.push('\n');
            cur_meta = Some((caps[1].to_string(), caps[3].to_string()));
        } else if cur_meta.is_some() {
            cur_raw.push_str(line);
            cur_raw.push('\n');
            cur_body.push(line.to_string());
        }
    }

    if let Some((thread_id, source_file)) = cur_meta.take() {
        blocks.push(finalize_block(cur_raw, thread_id, source_file, cur_body));
    }

    blocks
}

fn finalize_block(
    raw_text: String,
    thread_id: String,
    source_file: String,
    body_lines: Vec<String>,
) -> RawBlock {
    let function_name = {
        let name = source_file.strip_suffix(".c").unwrap_or(&source_file);
        if name == "__handles" {
            "SQLAllocHandle".to_string()
        } else {
            name.to_string()
        }
    };

    let exit_dir_re = &*EXIT_DIR_RE;
    let mut direction = Direction::Enter;
    let mut env_addr = None;
    let mut conn_addr = None;
    let mut stmt_addr = None;
    let mut handle_type = None;
    let mut input_handle = None;
    let mut output_handle = None;

    for line in &body_lines {
        let trimmed = line.trim();
        if trimmed == "Entry:" {
            direction = Direction::Enter;
        } else if exit_dir_re.is_match(trimmed) {
            direction = Direction::Exit;
        } else if let Some(rest) = trimmed.strip_prefix("Environment = ") {
            env_addr = extract_addr(rest);
        } else if let Some(rest) = trimmed.strip_prefix("Connection = ") {
            conn_addr = extract_addr(rest);
        } else if let Some(rest) = trimmed.strip_prefix("Statement = ") {
            stmt_addr = extract_addr(rest);
        } else if let Some(rest) = trimmed.strip_prefix("Handle Type = ") {
            handle_type = rest.trim().parse::<i64>().ok();
        } else if let Some(rest) = trimmed.strip_prefix("Input Handle = ") {
            input_handle = extract_addr(rest);
        } else if let Some(rest) = trimmed.strip_prefix("Output Handle = ") {
            output_handle = extract_addr(rest);
        }
    }

    RawBlock {
        raw_text,
        thread_id,
        function_name,
        direction,
        scope: HandleScope::Unknown,
        env_addr,
        conn_addr,
        stmt_addr,
        handle_type,
        input_handle,
        output_handle,
    }
}

fn extract_addr(s: &str) -> Option<String> {
    let trimmed = s.trim();
    Some(trimmed.to_string())
}

// -- Handle hierarchy --

struct HandleHierarchy {
    parent_of: HashMap<String, String>,
    type_of: HashMap<String, HandleType>,
    name_of: HashMap<String, String>,
    env_counter: usize,
    conn_counter: usize,
    stmt_counter: usize,
}

impl HandleHierarchy {
    fn new() -> Self {
        Self {
            parent_of: HashMap::new(),
            type_of: HashMap::new(),
            name_of: HashMap::new(),
            env_counter: 0,
            conn_counter: 0,
            stmt_counter: 0,
        }
    }

    fn register(&mut self, handle_type: HandleType, parent_key: &str, child_key: &str) {
        if self.type_of.contains_key(child_key) {
            return;
        }
        self.parent_of
            .insert(child_key.to_string(), parent_key.to_string());
        self.type_of.insert(child_key.to_string(), handle_type);

        let name = match handle_type {
            HandleType::Env => {
                let n = format!("env_{}", self.env_counter);
                self.env_counter += 1;
                n
            }
            HandleType::Dbc => {
                let n = format!("conn_{}", self.conn_counter);
                self.conn_counter += 1;
                n
            }
            HandleType::Stmt => {
                let n = format!("stmt_{}", self.stmt_counter);
                self.stmt_counter += 1;
                n
            }
            HandleType::Desc => {
                format!("desc_{child_key}")
            }
        };
        self.name_of.insert(child_key.to_string(), name);
    }

    fn env_of<'a>(&'a self, addr: &'a str) -> Option<&'a str> {
        match self.type_of.get(addr)? {
            HandleType::Env => Some(addr),
            HandleType::Dbc => self.parent_of.get(addr).map(|s| s.as_str()),
            HandleType::Stmt => {
                let conn = self.parent_of.get(addr)?;
                self.parent_of.get(conn.as_str()).map(|s| s.as_str())
            }
            HandleType::Desc => None,
        }
    }

    fn conn_of<'a>(&'a self, addr: &'a str) -> Option<&'a str> {
        match self.type_of.get(addr)? {
            HandleType::Dbc => Some(addr),
            HandleType::Stmt => self.parent_of.get(addr).map(|s| s.as_str()),
            _ => None,
        }
    }

    fn name<'a>(&'a self, addr: &'a str) -> &'a str {
        self.name_of.get(addr).map(|s| s.as_str()).unwrap_or(addr)
    }

    fn all_envs(&self) -> Vec<String> {
        self.type_of
            .iter()
            .filter(|(_, t)| **t == HandleType::Env)
            .map(|(a, _)| a.clone())
            .collect()
    }

    fn all_conns(&self) -> Vec<String> {
        self.type_of
            .iter()
            .filter(|(_, t)| **t == HandleType::Dbc)
            .map(|(a, _)| a.clone())
            .collect()
    }

    fn all_stmts(&self) -> Vec<String> {
        self.type_of
            .iter()
            .filter(|(_, t)| **t == HandleType::Stmt)
            .map(|(a, _)| a.clone())
            .collect()
    }

    fn conn_has_stmts(&self, conn_addr: &str) -> bool {
        self.type_of.iter().any(|(addr, t)| {
            *t == HandleType::Stmt
                && self.parent_of.get(addr).map(|s| s.as_str()) == Some(conn_addr)
        })
    }

    fn env_has_stmts(&self, env_addr: &str) -> bool {
        self.all_conns().iter().any(|conn_addr| {
            self.env_of(conn_addr) == Some(env_addr) && self.conn_has_stmts(conn_addr)
        })
    }
}

fn versioned_key(addr: &str, generation: usize) -> String {
    if generation == 0 {
        addr.to_string()
    } else {
        format!("{addr}#{generation}")
    }
}

fn build_hierarchy_and_assign_scopes(blocks: &mut [RawBlock]) -> HandleHierarchy {
    let mut hierarchy = HandleHierarchy::new();
    let mut addr_generation: HashMap<String, usize> = HashMap::new();
    let mut active_handles: HashMap<String, String> = HashMap::new();
    let mut pending_scopes: HashMap<(String, String), Vec<HandleScope>> = HashMap::new();

    struct PendingAlloc {
        handle_type: i64,
        input_handle: String,
    }
    let mut pending_allocs: HashMap<String, Vec<PendingAlloc>> = HashMap::new();

    for block in blocks.iter_mut() {
        // Step 1: Process alloc events to register handles before scope determination.
        if block.function_name == "SQLAllocHandle" {
            match block.direction {
                Direction::Enter => {
                    if let (Some(ht), Some(ref ih)) = (block.handle_type, &block.input_handle) {
                        pending_allocs
                            .entry(block.thread_id.clone())
                            .or_default()
                            .push(PendingAlloc {
                                handle_type: ht,
                                input_handle: ih.clone(),
                            });
                    }
                }
                Direction::Exit => {
                    if let Some(ref env_addr) = block.env_addr {
                        let gen = addr_generation.entry(env_addr.clone()).or_insert(0);
                        let key = versioned_key(env_addr, *gen);
                        if !hierarchy.type_of.contains_key(&key) {
                            hierarchy.register(HandleType::Env, "SQL_NULL_HANDLE", &key);
                        }
                        active_handles.insert(env_addr.clone(), key);
                    } else if let Some(ref out_addr) = block.output_handle {
                        let alloc = pending_allocs
                            .get_mut(&block.thread_id)
                            .and_then(|stack| stack.pop());
                        if let Some(alloc) = alloc {
                            if let Some(ht) = HandleType::from_value(alloc.handle_type) {
                                let gen = addr_generation.entry(out_addr.clone()).or_insert(0);
                                let key = versioned_key(out_addr, *gen);
                                let parent_key = active_handles
                                    .get(&alloc.input_handle)
                                    .cloned()
                                    .unwrap_or_else(|| alloc.input_handle.clone());
                                hierarchy.register(ht, &parent_key, &key);
                                active_handles.insert(out_addr.clone(), key);
                            }
                        }
                    }
                }
            }
        }

        // Step 2: Determine scope using current active_handles mapping.
        let scope = determine_scope(block, &hierarchy, &active_handles);

        // Step 3: Assign scope (with pending-scope inheritance for Exit blocks).
        match block.direction {
            Direction::Enter => {
                block.scope = scope.clone();
                pending_scopes
                    .entry((block.thread_id.clone(), block.function_name.clone()))
                    .or_default()
                    .push(scope);
            }
            Direction::Exit => {
                if matches!(scope, HandleScope::Unknown) {
                    let inherited = pending_scopes
                        .get_mut(&(block.thread_id.clone(), block.function_name.clone()))
                        .and_then(|stack| stack.pop());
                    block.scope = inherited.unwrap_or(HandleScope::Unknown);
                } else {
                    pending_scopes
                        .get_mut(&(block.thread_id.clone(), block.function_name.clone()))
                        .and_then(|stack| stack.pop());
                    block.scope = scope;
                }
            }
        }

        // Step 4: Process free events AFTER scope determination so the handle
        // is still resolvable during scope lookup for the free block itself.
        if block.function_name == "SQLFreeHandle" && block.direction == Direction::Enter {
            if let Some(ref ih) = block.input_handle {
                active_handles.remove(ih);
                let gen = addr_generation.entry(ih.clone()).or_insert(0);
                *gen += 1;
            }
        }
    }

    hierarchy
}

fn determine_scope(
    block: &RawBlock,
    hierarchy: &HandleHierarchy,
    active_handles: &HashMap<String, String>,
) -> HandleScope {
    if let Some(ref addr) = block.stmt_addr {
        let key = active_handles
            .get(addr)
            .cloned()
            .unwrap_or_else(|| addr.clone());
        return HandleScope::Statement(key);
    }
    if let Some(ref addr) = block.conn_addr {
        let key = active_handles
            .get(addr)
            .cloned()
            .unwrap_or_else(|| addr.clone());
        return HandleScope::Connection(key);
    }
    if let Some(ref addr) = block.env_addr {
        let key = active_handles
            .get(addr)
            .cloned()
            .unwrap_or_else(|| addr.clone());
        return HandleScope::Env(key);
    }

    let is_alloc_or_free =
        block.function_name == "SQLAllocHandle" || block.function_name == "SQLFreeHandle";

    if !is_alloc_or_free {
        return HandleScope::Unknown;
    }

    if let Some(ref out_addr) = block.output_handle {
        let key = active_handles
            .get(out_addr)
            .cloned()
            .unwrap_or_else(|| out_addr.clone());
        if let Some(ht) = hierarchy.type_of.get(&key) {
            return match ht {
                HandleType::Env => HandleScope::Env(key),
                HandleType::Dbc => HandleScope::Connection(key),
                HandleType::Stmt => HandleScope::Statement(key),
                HandleType::Desc => HandleScope::Unknown,
            };
        }
    }

    if let (Some(ht_val), Some(ref ih)) = (block.handle_type, &block.input_handle) {
        let key = active_handles
            .get(ih)
            .cloned()
            .unwrap_or_else(|| ih.clone());
        if block.function_name == "SQLFreeHandle" {
            return match HandleType::from_value(ht_val) {
                Some(HandleType::Env) => HandleScope::Env(key),
                Some(HandleType::Dbc) => HandleScope::Connection(key),
                Some(HandleType::Stmt) => HandleScope::Statement(key),
                _ => HandleScope::Unknown,
            };
        }

        return match HandleType::from_value(ht_val) {
            Some(HandleType::Dbc) => HandleScope::Env(key),
            Some(HandleType::Stmt) => HandleScope::Connection(key),
            _ => HandleScope::Unknown,
        };
    }

    HandleScope::Unknown
}

// -- Output writing --

fn write_split_output(
    blocks: &[RawBlock],
    hierarchy: &HandleHierarchy,
    output_dir: &Path,
    mode: SplitMode,
    require_statements: bool,
) -> io::Result<SplitStats> {
    // Group block indices by scope addr (env/conn/stmt)
    let mut env_block_indices: HashMap<String, Vec<usize>> = HashMap::new();
    let mut conn_block_indices: HashMap<String, Vec<usize>> = HashMap::new();
    let mut stmt_block_indices: HashMap<String, Vec<usize>> = HashMap::new();

    for (i, block) in blocks.iter().enumerate() {
        match &block.scope {
            HandleScope::Env(addr) => {
                env_block_indices.entry(addr.clone()).or_default().push(i);
            }
            HandleScope::Connection(addr) => {
                conn_block_indices.entry(addr.clone()).or_default().push(i);
            }
            HandleScope::Statement(addr) => {
                stmt_block_indices.entry(addr.clone()).or_default().push(i);
            }
            HandleScope::Unknown => {}
        }
    }

    let mut files_written = 0;

    match mode {
        SplitMode::Env => {
            for env_addr in hierarchy.all_envs() {
                if require_statements && !hierarchy.env_has_stmts(&env_addr) {
                    continue;
                }
                let env_name = hierarchy.name(&env_addr);
                let dir = output_dir.join(env_name);
                fs::create_dir_all(&dir)?;

                let mut indices: Vec<usize> = Vec::new();

                if let Some(ei) = env_block_indices.get(&env_addr) {
                    indices.extend(ei);
                }
                for conn_addr in hierarchy.all_conns() {
                    if hierarchy.env_of(&conn_addr) == Some(env_addr.as_str()) {
                        if let Some(ci) = conn_block_indices.get(&conn_addr) {
                            indices.extend(ci);
                        }
                        for stmt_addr in hierarchy.all_stmts() {
                            if hierarchy.conn_of(&stmt_addr) == Some(conn_addr.as_str()) {
                                if let Some(si) = stmt_block_indices.get(&stmt_addr) {
                                    indices.extend(si);
                                }
                            }
                        }
                    }
                }

                indices.sort_unstable();
                write_blocks_to_file(&dir.join("trace.txt"), blocks, &indices)?;
                files_written += 1;
            }
        }

        SplitMode::Connection => {
            for conn_addr in hierarchy.all_conns() {
                if require_statements && !hierarchy.conn_has_stmts(&conn_addr) {
                    continue;
                }
                let conn_name = hierarchy.name(&conn_addr);
                let env_addr = hierarchy.env_of(&conn_addr).unwrap_or("unknown_env");
                let env_name = hierarchy.name(env_addr);
                let dir = output_dir.join(env_name).join(conn_name);
                fs::create_dir_all(&dir)?;

                let mut indices: Vec<usize> = Vec::new();

                // Repeat env blocks
                if let Some(ei) = env_block_indices.get(env_addr) {
                    indices.extend(ei);
                }
                // Connection blocks
                if let Some(ci) = conn_block_indices.get(&conn_addr) {
                    indices.extend(ci);
                }
                // Child statement blocks
                for stmt_addr in hierarchy.all_stmts() {
                    if hierarchy.conn_of(&stmt_addr) == Some(conn_addr.as_str()) {
                        if let Some(si) = stmt_block_indices.get(&stmt_addr) {
                            indices.extend(si);
                        }
                    }
                }

                indices.sort_unstable();
                write_blocks_to_file(&dir.join("trace.txt"), blocks, &indices)?;
                files_written += 1;
            }
        }

        SplitMode::Statement => {
            for stmt_addr in hierarchy.all_stmts() {
                let stmt_name = hierarchy.name(&stmt_addr);
                let conn_addr_str = hierarchy.conn_of(&stmt_addr).unwrap_or("unknown_conn");
                let conn_name = hierarchy.name(conn_addr_str);
                let env_addr_str = hierarchy.env_of(&stmt_addr).unwrap_or("unknown_env");
                let env_name = hierarchy.name(env_addr_str);

                let dir = output_dir.join(env_name).join(conn_name);
                fs::create_dir_all(&dir)?;

                let mut indices: Vec<usize> = Vec::new();

                // Repeat env blocks
                if let Some(ei) = env_block_indices.get(env_addr_str) {
                    indices.extend(ei);
                }
                // Repeat conn blocks
                if let Some(ci) = conn_block_indices.get(conn_addr_str) {
                    indices.extend(ci);
                }
                // Statement blocks
                if let Some(si) = stmt_block_indices.get(&stmt_addr) {
                    indices.extend(si);
                }

                indices.sort_unstable();
                let filename = format!("{}_trace.txt", stmt_name);
                write_blocks_to_file(&dir.join(filename), blocks, &indices)?;
                files_written += 1;
            }
        }
    }

    Ok(SplitStats {
        files_written,
        blocks_processed: blocks.len(),
        envs_found: hierarchy.all_envs().len(),
        connections_found: hierarchy.all_conns().len(),
        statements_found: hierarchy.all_stmts().len(),
    })
}

fn write_blocks_to_file(path: &Path, blocks: &[RawBlock], indices: &[usize]) -> io::Result<()> {
    use std::io::Write;
    let file = fs::File::create(path)?;
    let mut writer = io::BufWriter::new(file);
    for &idx in indices {
        writer.write_all(blocks[idx].raw_text.as_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn sample_trace() -> &'static str {
        "\
[ODBC][100][1000.000000][__handles.c][499]
\t\tExit:[SQL_SUCCESS]
\t\t\tEnvironment = 0xenv1
[ODBC][100][1000.000001][SQLSetEnvAttr.c][189]
\t\tEntry:
\t\t\tEnvironment = 0xenv1
\t\t\tAttribute = SQL_ATTR_ODBC_VERSION
\t\t\tValue = 0x17c
\t\t\tStrLen = 0
[ODBC][100][1000.000002][SQLSetEnvAttr.c][381]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][1000.000003][SQLAllocHandle.c][395]
\t\tEntry:
\t\t\tHandle Type = 2
\t\t\tInput Handle = 0xenv1
[ODBC][100][1000.000004][SQLAllocHandle.c][531]
\t\tExit:[SQL_SUCCESS]
\t\t\tOutput Handle = 0xconn1
[ODBC][100][1000.000005][SQLDriverConnect.c][751]
\t\tEntry:
\t\t\tConnection = 0xconn1
\t\t\tStr In = [DSN=test][length = 8]
[ODBC][100][1000.000006][SQLDriverConnect.c][1809]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][1000.000007][SQLAllocHandle.c][578]
\t\tEntry:
\t\t\tHandle Type = 3
\t\t\tInput Handle = 0xconn1
[ODBC][100][1000.000008][SQLAllocHandle.c][1123]
\t\tExit:[SQL_SUCCESS]
\t\t\tOutput Handle = 0xstmt1
[ODBC][100][1000.000009][SQLExecDirect.c][240]
\t\tEntry:
\t\t\tStatement = 0xstmt1
\t\t\tSQL = [SELECT 1][length = 8 (SQL_NTS)]
[ODBC][100][1000.000010][SQLExecDirect.c][521]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][1000.000011][SQLAllocHandle.c][578]
\t\tEntry:
\t\t\tHandle Type = 3
\t\t\tInput Handle = 0xconn1
[ODBC][100][1000.000012][SQLAllocHandle.c][1123]
\t\tExit:[SQL_SUCCESS]
\t\t\tOutput Handle = 0xstmt2
[ODBC][100][1000.000013][SQLExecDirect.c][240]
\t\tEntry:
\t\t\tStatement = 0xstmt2
\t\t\tSQL = [SELECT 2][length = 8 (SQL_NTS)]
[ODBC][100][1000.000014][SQLExecDirect.c][521]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][1000.000015][SQLFreeHandle.c][387]
\t\tEntry:
\t\t\tHandle Type = 3
\t\t\tInput Handle = 0xstmt1
[ODBC][100][1000.000016][SQLFreeHandle.c][490]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][1000.000017][SQLFreeHandle.c][387]
\t\tEntry:
\t\t\tHandle Type = 3
\t\t\tInput Handle = 0xstmt2
[ODBC][100][1000.000018][SQLFreeHandle.c][490]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][1000.000019][SQLDisconnect.c][208]
\t\tEntry:
\t\t\tConnection = 0xconn1
[ODBC][100][1000.000020][SQLDisconnect.c][358]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][1000.000021][SQLFreeHandle.c][290]
\t\tEntry:
\t\t\tHandle Type = 2
\t\t\tInput Handle = 0xconn1
[ODBC][100][1000.000022][SQLFreeHandle.c][339]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][1000.000023][SQLFreeHandle.c][220]
\t\tEntry:
\t\t\tHandle Type = 1
\t\t\tInput Handle = 0xenv1
[ODBC][100][1000.000024][SQLFreeHandle.c][250]
\t\tExit:[SQL_SUCCESS]
"
    }

    #[test]
    fn test_extract_raw_blocks() {
        let blocks = extract_raw_blocks(sample_trace());
        assert!(!blocks.is_empty());
        assert_eq!(blocks[0].function_name, "SQLAllocHandle");
        assert_eq!(blocks[0].direction, Direction::Exit);
        assert_eq!(blocks[0].env_addr, Some("0xenv1".to_string()));
    }

    #[test]
    fn test_build_hierarchy() {
        let mut blocks = extract_raw_blocks(sample_trace());
        let h = build_hierarchy_and_assign_scopes(&mut blocks);

        assert!(h.type_of.contains_key("0xenv1"));
        assert_eq!(h.type_of["0xenv1"], HandleType::Env);

        assert!(h.type_of.contains_key("0xconn1"));
        assert_eq!(h.type_of["0xconn1"], HandleType::Dbc);
        assert_eq!(h.parent_of["0xconn1"], "0xenv1");

        assert!(h.type_of.contains_key("0xstmt1"));
        assert_eq!(h.type_of["0xstmt1"], HandleType::Stmt);
        assert_eq!(h.parent_of["0xstmt1"], "0xconn1");

        assert!(h.type_of.contains_key("0xstmt2"));
    }

    #[test]
    fn test_scope_assignment() {
        let mut blocks = extract_raw_blocks(sample_trace());
        let _h = build_hierarchy_and_assign_scopes(&mut blocks);

        // __handles.c Exit → env scope
        assert!(matches!(&blocks[0].scope, HandleScope::Env(a) if a == "0xenv1"));

        // SQLSetEnvAttr Entry → env scope
        assert!(matches!(&blocks[1].scope, HandleScope::Env(a) if a == "0xenv1"));

        // SQLSetEnvAttr Exit → env scope (inherited from Entry)
        assert!(matches!(&blocks[2].scope, HandleScope::Env(a) if a == "0xenv1"));

        // SQLDriverConnect Entry → conn scope
        let dc_entry = blocks
            .iter()
            .find(|b| b.function_name == "SQLDriverConnect" && b.direction == Direction::Enter)
            .unwrap();
        assert!(matches!(&dc_entry.scope, HandleScope::Connection(a) if a == "0xconn1"));

        // SQLExecDirect Entry for stmt1 → stmt scope
        let exec1 = blocks
            .iter()
            .find(|b| {
                b.function_name == "SQLExecDirect"
                    && b.direction == Direction::Enter
                    && b.stmt_addr.as_deref() == Some("0xstmt1")
            })
            .unwrap();
        assert!(matches!(&exec1.scope, HandleScope::Statement(a) if a == "0xstmt1"));
    }

    #[test]
    fn test_split_by_env() {
        let tmp = TempDir::new().unwrap();
        let input = write_temp_trace(tmp.path(), sample_trace());

        let stats = split_trace(&input, &tmp.path().join("out"), SplitMode::Env, false).unwrap();

        assert_eq!(stats.envs_found, 1);
        assert_eq!(stats.files_written, 1);

        let trace = fs::read_to_string(tmp.path().join("out/env_0/trace.txt")).unwrap();
        assert!(trace.contains("0xenv1"));
        assert!(trace.contains("0xconn1"));
        assert!(trace.contains("0xstmt1"));
    }

    #[test]
    fn test_split_by_connection() {
        let tmp = TempDir::new().unwrap();
        let input = write_temp_trace(tmp.path(), sample_trace());

        let stats = split_trace(
            &input,
            &tmp.path().join("out"),
            SplitMode::Connection,
            false,
        )
        .unwrap();

        assert_eq!(stats.connections_found, 1);
        assert_eq!(stats.files_written, 1);

        let trace = fs::read_to_string(tmp.path().join("out/env_0/conn_0/trace.txt")).unwrap();
        // Should include env blocks (repeated) + conn blocks + stmt blocks
        assert!(trace.contains("0xenv1"));
        assert!(trace.contains("0xconn1"));
        assert!(trace.contains("0xstmt1"));
        assert!(trace.contains("0xstmt2"));
    }

    #[test]
    fn test_split_by_statement() {
        let tmp = TempDir::new().unwrap();
        let input = write_temp_trace(tmp.path(), sample_trace());

        let stats =
            split_trace(&input, &tmp.path().join("out"), SplitMode::Statement, false).unwrap();

        assert_eq!(stats.statements_found, 2);
        assert_eq!(stats.files_written, 2);

        let trace1 =
            fs::read_to_string(tmp.path().join("out/env_0/conn_0/stmt_0_trace.txt")).unwrap();
        let trace2 =
            fs::read_to_string(tmp.path().join("out/env_0/conn_0/stmt_1_trace.txt")).unwrap();

        // Both should include env and conn blocks (repeated)
        assert!(trace1.contains("0xenv1"));
        assert!(trace1.contains("0xconn1"));
        assert!(trace2.contains("0xenv1"));
        assert!(trace2.contains("0xconn1"));

        // Each should include only its own stmt blocks
        assert!(trace1.contains("0xstmt1"));
        assert!(!trace1.contains("0xstmt2"));
        assert!(trace2.contains("0xstmt2"));
        assert!(!trace2.contains("0xstmt1"));
    }

    fn sample_trace_address_reuse() -> &'static str {
        "\
[ODBC][100][1000.000000][__handles.c][499]
\t\tExit:[SQL_SUCCESS]
\t\t\tEnvironment = 0xenv1
[ODBC][100][1000.000001][SQLSetEnvAttr.c][189]
\t\tEntry:
\t\t\tEnvironment = 0xenv1
\t\t\tAttribute = SQL_ATTR_ODBC_VERSION
\t\t\tValue = 0x17c
\t\t\tStrLen = 0
[ODBC][100][1000.000002][SQLSetEnvAttr.c][381]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][1000.000003][SQLAllocHandle.c][395]
\t\tEntry:
\t\t\tHandle Type = 2
\t\t\tInput Handle = 0xenv1
[ODBC][100][1000.000004][SQLAllocHandle.c][531]
\t\tExit:[SQL_SUCCESS]
\t\t\tOutput Handle = 0xconn1
[ODBC][100][1000.000005][SQLDriverConnect.c][751]
\t\tEntry:
\t\t\tConnection = 0xconn1
\t\t\tStr In = [DSN=test][length = 8]
[ODBC][100][1000.000006][SQLDriverConnect.c][1809]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][1000.000007][SQLAllocHandle.c][578]
\t\tEntry:
\t\t\tHandle Type = 3
\t\t\tInput Handle = 0xconn1
[ODBC][100][1000.000008][SQLAllocHandle.c][1123]
\t\tExit:[SQL_SUCCESS]
\t\t\tOutput Handle = 0xstmt1
[ODBC][100][1000.000009][SQLExecDirect.c][240]
\t\tEntry:
\t\t\tStatement = 0xstmt1
\t\t\tSQL = [SELECT 1][length = 8 (SQL_NTS)]
[ODBC][100][1000.000010][SQLExecDirect.c][521]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][1000.000011][SQLFreeHandle.c][387]
\t\tEntry:
\t\t\tHandle Type = 3
\t\t\tInput Handle = 0xstmt1
[ODBC][100][1000.000012][SQLFreeHandle.c][490]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][1000.000013][SQLDisconnect.c][208]
\t\tEntry:
\t\t\tConnection = 0xconn1
[ODBC][100][1000.000014][SQLDisconnect.c][358]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][1000.000015][SQLFreeHandle.c][290]
\t\tEntry:
\t\t\tHandle Type = 2
\t\t\tInput Handle = 0xconn1
[ODBC][100][1000.000016][SQLFreeHandle.c][339]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][1000.000017][SQLFreeHandle.c][220]
\t\tEntry:
\t\t\tHandle Type = 1
\t\t\tInput Handle = 0xenv1
[ODBC][100][1000.000018][SQLFreeHandle.c][250]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][2000.000000][__handles.c][499]
\t\tExit:[SQL_SUCCESS]
\t\t\tEnvironment = 0xenv1
[ODBC][100][2000.000001][SQLSetEnvAttr.c][189]
\t\tEntry:
\t\t\tEnvironment = 0xenv1
\t\t\tAttribute = SQL_ATTR_ODBC_VERSION
\t\t\tValue = 0x17c
\t\t\tStrLen = 0
[ODBC][100][2000.000002][SQLSetEnvAttr.c][381]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][2000.000003][SQLAllocHandle.c][395]
\t\tEntry:
\t\t\tHandle Type = 2
\t\t\tInput Handle = 0xenv1
[ODBC][100][2000.000004][SQLAllocHandle.c][531]
\t\tExit:[SQL_SUCCESS]
\t\t\tOutput Handle = 0xconn1
[ODBC][100][2000.000005][SQLDriverConnect.c][751]
\t\tEntry:
\t\t\tConnection = 0xconn1
\t\t\tStr In = [DSN=test][length = 8]
[ODBC][100][2000.000006][SQLDriverConnect.c][1809]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][2000.000007][SQLAllocHandle.c][578]
\t\tEntry:
\t\t\tHandle Type = 3
\t\t\tInput Handle = 0xconn1
[ODBC][100][2000.000008][SQLAllocHandle.c][1123]
\t\tExit:[SQL_SUCCESS]
\t\t\tOutput Handle = 0xstmt1
[ODBC][100][2000.000009][SQLExecDirect.c][240]
\t\tEntry:
\t\t\tStatement = 0xstmt1
\t\t\tSQL = [SELECT 2][length = 8 (SQL_NTS)]
[ODBC][100][2000.000010][SQLExecDirect.c][521]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][2000.000011][SQLFreeHandle.c][387]
\t\tEntry:
\t\t\tHandle Type = 3
\t\t\tInput Handle = 0xstmt1
[ODBC][100][2000.000012][SQLFreeHandle.c][490]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][2000.000013][SQLDisconnect.c][208]
\t\tEntry:
\t\t\tConnection = 0xconn1
[ODBC][100][2000.000014][SQLDisconnect.c][358]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][2000.000015][SQLFreeHandle.c][290]
\t\tEntry:
\t\t\tHandle Type = 2
\t\t\tInput Handle = 0xconn1
[ODBC][100][2000.000016][SQLFreeHandle.c][339]
\t\tExit:[SQL_SUCCESS]
[ODBC][100][2000.000017][SQLFreeHandle.c][220]
\t\tEntry:
\t\t\tHandle Type = 1
\t\t\tInput Handle = 0xenv1
[ODBC][100][2000.000018][SQLFreeHandle.c][250]
\t\tExit:[SQL_SUCCESS]
"
    }

    #[test]
    fn test_address_reuse_hierarchy() {
        let mut blocks = extract_raw_blocks(sample_trace_address_reuse());
        let h = build_hierarchy_and_assign_scopes(&mut blocks);

        assert_eq!(h.all_envs().len(), 2, "Should find 2 envs");
        assert_eq!(h.all_conns().len(), 2, "Should find 2 connections");
        assert_eq!(h.all_stmts().len(), 2, "Should find 2 statements");

        assert_eq!(h.type_of["0xenv1"], HandleType::Env);
        assert_eq!(h.type_of["0xenv1#1"], HandleType::Env);
        assert_eq!(h.type_of["0xconn1"], HandleType::Dbc);
        assert_eq!(h.type_of["0xconn1#1"], HandleType::Dbc);
        assert_eq!(h.type_of["0xstmt1"], HandleType::Stmt);
        assert_eq!(h.type_of["0xstmt1#1"], HandleType::Stmt);

        assert_eq!(h.parent_of["0xconn1"], "0xenv1");
        assert_eq!(h.parent_of["0xconn1#1"], "0xenv1#1");
        assert_eq!(h.parent_of["0xstmt1"], "0xconn1");
        assert_eq!(h.parent_of["0xstmt1#1"], "0xconn1#1");
    }

    #[test]
    fn test_address_reuse_split_by_env() {
        let tmp = TempDir::new().unwrap();
        let input = write_temp_trace(tmp.path(), sample_trace_address_reuse());

        let stats = split_trace(&input, &tmp.path().join("out"), SplitMode::Env, false).unwrap();

        assert_eq!(
            stats.envs_found, 2,
            "Should find 2 envs (same address, different lifetimes)"
        );
        assert_eq!(stats.files_written, 2);

        let trace0 = fs::read_to_string(tmp.path().join("out/env_0/trace.txt")).unwrap();
        let trace1 = fs::read_to_string(tmp.path().join("out/env_1/trace.txt")).unwrap();

        assert!(
            trace0.contains("SELECT 1"),
            "First env should contain SELECT 1"
        );
        assert!(
            !trace0.contains("SELECT 2"),
            "First env should not contain SELECT 2"
        );
        assert!(
            trace1.contains("SELECT 2"),
            "Second env should contain SELECT 2"
        );
        assert!(
            !trace1.contains("SELECT 1"),
            "Second env should not contain SELECT 1"
        );
    }

    #[test]
    fn test_address_reuse_split_by_stmt() {
        let tmp = TempDir::new().unwrap();
        let input = write_temp_trace(tmp.path(), sample_trace_address_reuse());

        let stats =
            split_trace(&input, &tmp.path().join("out"), SplitMode::Statement, false).unwrap();

        assert_eq!(stats.statements_found, 2, "Should find 2 statements");
        assert_eq!(stats.files_written, 2);
    }

    fn write_temp_trace(dir: &Path, content: &str) -> PathBuf {
        let path = dir.join("input_trace.txt");
        fs::write(&path, content).unwrap();
        path
    }
}
