# Node.js WIF Artifact Builder

This local-only image builds the linux/amd64 Node.js artifact used by the WIF
end-to-end runner. It contains what Node e2e needs to build that artifact. The
repository is bind-mounted at runtime; it is not baked into the image.

## Building

```bash
./tests/docker/wif-nodejs/build.sh
```

`PLATFORM` defaults to `linux/amd64`, and `IMAGE_TAG` defaults to
`ud-wif-nodejs-local:latest`.

`tests/auth/run_wif_local.sh nodejs` builds this image and the artifact automatically
unless `--skip-build` is passed.

Jenkins does not use this image. CI builds the Node artifact with the
Artifactory image named by `NODEJS_DOCKER_IMAGE`. The WIF VMs run that
artifact in a public Node runtime image.
