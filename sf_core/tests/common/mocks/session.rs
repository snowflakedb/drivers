/// Check whether a wiremock request is a logout (DELETE session) request.
pub fn is_logout_request(r: &wiremock::Request) -> bool {
    r.url.path() == "/session"
        && r.url
            .query()
            .map(|q| q.contains("delete=true"))
            .unwrap_or(false)
}
