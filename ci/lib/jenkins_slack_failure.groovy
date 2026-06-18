#!/usr/bin/env groovy

import groovy.json.JsonOutput

def notifyFailure(Map opts = [:]) {
    String result = (currentBuild.currentResult ?: '').trim()
    if (result != 'FAILURE') {
        echo "Slack failure notification skipped: result=${result}"
        return
    }

    // Fire only on push-to-main and merge-queue builds, mirroring the GitHub
    // Actions notifier (regular PR runs are silently ignored).
    String branch = _effectiveBranch(opts)
    boolean onMain = branch == 'main'
    boolean onMergeQueue = branch.startsWith('gh-readonly-queue/')
    if (!onMain && !onMergeQueue) {
        echo "Slack failure notification skipped: branch=${branch}"
        return
    }

    String workspace = env.WORKSPACE ?: pwd()
    String payloadFile = "${workspace}/.jenkins/slack_failure_payload.json"
    String credentialId = (opts.credentialId ?: env.SLACK_BOT_TOKEN_CREDENTIAL_ID ?: 'drivers_review_bot_token').toString()
    String channel = (opts.channel ?: env.SLACK_CHANNEL ?: 'universal-driver-ci-alerts').toString()
    String workflowName = (opts.workflowName ?: env.JOB_NAME ?: 'Jenkins pipeline').toString()
    String runUrl = (env.BUILD_URL ?: '').toString()
    String triggerEvent = onMergeQueue ? 'merge_group' : 'push'

    Map gitMeta = _gitMetadata()
    List<Map<String, String>> failedJobs = _failedJobsForPayload(runUrl)
    String failedJobsJson = JsonOutput.toJson(failedJobs)

    def postToSlack = {
        echo "Slack failure notification: branch=${branch}, channel=${channel}, credentialId=${credentialId}, runUrl=${runUrl}"
        sh(
            label: 'Build Slack failure payload',
            script: """
                set -eu
                python3 scripts/jenkins_ci_failure_slack_payload.py \
                  --workflow-name ${_shellQuote(workflowName)} \
                  --run-url ${_shellQuote(runUrl)} \
                  --head-sha ${_shellQuote(gitMeta.headSha)} \
                  --head-branch ${_shellQuote(branch)} \
                  --trigger-event ${_shellQuote(triggerEvent)} \
                  --failed-jobs-json ${_shellQuote(failedJobsJson)} \
                  --author-email ${_shellQuote(gitMeta.authorEmail)} \
                  --author-name ${_shellQuote(gitMeta.authorName)} \
                  --slack-channel ${_shellQuote(channel)} \
                  --artifacts-root ${_shellQuote(workspace)} \
                  --payload-file ${_shellQuote(payloadFile)}
            """
        )
        sh(
            label: 'Post CI failure to Slack',
            script: """
                set -eu
                PAYLOAD_FILE=${_shellQuote(payloadFile)}
                RESPONSE_FILE="${workspace}/.jenkins/slack_post_response.json"
                curl -sS --fail \
                  -X POST \
                  -H "Authorization: Bearer \$SLACK_BOT_TOKEN" \
                  -H "Content-Type: application/json; charset=utf-8" \
                  "https://slack.com/api/chat.postMessage" \
                  --data "@\${PAYLOAD_FILE}" \
                  > "\${RESPONSE_FILE}"
                python3 - "\${RESPONSE_FILE}" <<'PYEOF'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as handle:
    data = json.load(handle)
print(
    "Slack API response: "
    f"ok={data.get('ok')} "
    f"error={data.get('error')} "
    f"channel={data.get('channel')} "
    f"ts={data.get('ts')}"
)
if not data.get("ok"):
    raise SystemExit(f"Slack API returned ok=false: {data!r}")
PYEOF
            """
        )
    }

    withCredentials([string(credentialsId: credentialId, variable: 'SLACK_BOT_TOKEN')]) {
        postToSlack()
    }
}

String _effectiveBranch(Map opts) {
    return (opts.branch ?: env.BRANCH_NAME ?: env.CHANGE_BRANCH ?: '').toString()
}

Map _gitMetadata() {
    // Reached only on push-to-main and merge-queue builds (see the gate in
    // notifyFailure). Neither is a PR merge checkout, so HEAD is the real commit
    // — this mirrors the GitHub Actions notifier's head_commit.author.
    String sha = (env.GIT_COMMIT ?: '').toString()
    String authorEmail = ''
    String authorName = ''

    try {
        sha = sh(returnStdout: true, script: 'git rev-parse HEAD').trim()
    } catch (Exception ignored) {
        echo "Unable to resolve git SHA from checkout; using env.GIT_COMMIT."
    }
    try {
        authorEmail = sh(returnStdout: true, script: 'git log -1 --pretty=%ae').trim()
    } catch (Exception ignored) {
        echo "Unable to resolve commit author email from git."
    }
    try {
        authorName = sh(returnStdout: true, script: 'git log -1 --pretty=%an').trim()
    } catch (Exception ignored) {
        echo "Unable to resolve commit author name from git."
    }

    return [
        headSha    : sha,
        authorEmail: authorEmail,
        authorName : authorName,
    ]
}

List<Map<String, String>> _failedJobsForPayload(String runUrl) {
    String stageName = env.STAGE_NAME ?: 'Pipeline'
    return [[name: stageName, url: runUrl, failed_step: null]]
}

String _shellQuote(String value) {
    return "'" + (value ?: '').replace("'", "'\"'\"'") + "'"
}

return this
