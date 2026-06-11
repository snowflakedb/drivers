const { execSync } = require('child_process');

function processExists(processName) {
    try {
        const stdout = execSync(`ps -A | grep ${processName}`);
        return stdout.toString().split('\n').filter(line => line.includes(processName)).length > 0;
    } catch (error) {
        return false;
    }
}

function forceKillProcess(processName) {
    try {
        execSync(`pkill -9 -f ${processName}`, () => {});
    } catch (err) {
        // Continue
    }
}
async function main() {
    const timeout = Date.now() + 10000;
    while (Date.now() < timeout) {
        await forceKillProcess('chromium');
        await forceKillProcess('xdg-open');
        if (!processExists('chromium') && !processExists('xdg-open')) {
            break;
        }
    }
    process.exit(0);
}

main();
