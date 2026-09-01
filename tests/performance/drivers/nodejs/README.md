# Node.js perf driver

`app/` implements the same benchmark phases (connect, setup queries, fetch,
resource sampling) as the old driver's own benchmark code, ported as-is rather
than rewritten. It stays plain JavaScript instead of TypeScript for that
reason — porting it 1:1 keeps the comparison faithful to what the old driver's
benchmarks actually measured, and a rewrite risks introducing behavioral drift
between the two driver types this harness is meant to compare.

TODO: link the source this was ported from, for anyone who needs to look up
its history.

See `../../Jenkinsfile.pr-check`'s `Node.js` stage and `Dockerfile` in this
directory for how the two driver types (`universal`/`old`) get built and run.
