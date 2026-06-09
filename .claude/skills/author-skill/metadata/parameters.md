As part of this skill, we must run the `sf` command. Check whether
there exists an environment variable `AUTHOR_SKILL_SF_LOCATION`, and
to do this make sure to use the command
`printenv AUTHOR_SKILL_SF_LOCATION`.

If it is set, this points to the `sf` binary to run whenever the
skill needs to invoke `sf`. If the path is not set, you may just run
`sf` in the shell and let it resolve from PATH.

If we assigned a non-default `sf` location, tell the user that we are
using it (the path value) before the first invocation.
