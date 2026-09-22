use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::PyTzInfo;

pub(super) struct TimezoneProvider {
    name: Option<String>,
    tz: PyOnceLock<Py<PyTzInfo>>,
}

impl TimezoneProvider {
    pub(super) fn new(name: Option<String>) -> Self {
        Self {
            name,
            tz: PyOnceLock::new(),
        }
    }

    pub(super) fn get<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTzInfo>> {
        self.tz
            .get_or_try_init(py, || resolve_session_timezone(py, self.name.as_deref()))
            .map(|tz| tz.bind(py).clone())
    }
}

fn utc_tzinfo(py: Python<'_>) -> PyResult<Py<PyTzInfo>> {
    py.import("datetime")?
        .getattr("timezone")?
        .getattr("utc")?
        .cast_into::<PyTzInfo>()
        .map(|tz| tz.unbind())
        .map_err(|err| err.into())
}

fn resolve_session_timezone(
    py: Python<'_>,
    session_timezone: Option<&str>,
) -> PyResult<Py<PyTzInfo>> {
    let tz_name = session_timezone
        .filter(|name| !name.is_empty())
        .unwrap_or("UTC");
    let zoneinfo = py.import("zoneinfo")?;
    match zoneinfo.getattr("ZoneInfo")?.call1((tz_name,)) {
        Ok(zone) => zone
            .cast_into::<PyTzInfo>()
            .map(|tz| tz.unbind())
            .map_err(|err| err.into()),
        Err(err) => {
            let not_found = zoneinfo.getattr("ZoneInfoNotFoundError")?;
            if err.is_instance(py, &not_found) {
                tracing::warn!("converting to tzinfo failed");
                utc_tzinfo(py)
            } else {
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use pyo3::prelude::*;

    use super::TimezoneProvider;

    #[test]
    fn unknown_session_timezone_falls_back_to_utc() {
        Python::initialize();
        let tz = TimezoneProvider::new(Some("Not/AZone".to_string()));
        Python::attach(|py| {
            assert!(tz.tz.get(py).is_none());
            let resolved = tz.get(py).unwrap();
            let utc = py
                .import("datetime")
                .unwrap()
                .getattr("timezone")
                .unwrap()
                .getattr("utc")
                .unwrap();
            assert!(resolved.is(&utc));
            assert!(tz.tz.get(py).is_some());
        });
    }

    #[test]
    fn session_timezone_is_resolved_lazily() {
        Python::initialize();
        let tz = TimezoneProvider::new(None);
        Python::attach(|py| {
            assert!(tz.tz.get(py).is_none());
            let _ = tz.get(py).unwrap();
            assert!(tz.tz.get(py).is_some());
        });
    }
}
