---
category: bugfix
---

- Fixed GET from a stage without client-side encryption, such as an external S3 stage, failing with `data did not match any variant of untagged enum OneOrMany`. (snowflakedb/drivers#3226)
