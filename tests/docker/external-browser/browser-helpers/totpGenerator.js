const speakeasy = require('speakeasy');

const TOTP_STEP_SECONDS = 30;
const MIN_VALIDITY_SECONDS = 8;
const BOUNDARY_SLACK_MS = 200;

class TotpGenerator {
    constructor(seed) {
        this.seed = seed || process.env.SNOWFLAKE_AUTH_MFA_SEED;

        if (!this.seed) {
            throw new Error(
                'TOTP seed must be provided as parameter or set in SNOWFLAKE_AUTH_MFA_SEED environment variable'
            );
        }
    }

    /**
     * Generate only the currently valid TOTP code.
     *
     * Returning adjacent-window codes encouraged callers to submit a past or
     * not-yet-valid code immediately after a rejection. Each submission counts
     * toward the account's MFA lockout, so wait for a safe current window
     * instead of spraying fallback codes.
     *
     * Returns the bare token string. Does not write stdout — CLI main() prints
     * so library callers (Playwright) do not leak passcodes into Jenkins logs.
     */
    async generateTotp() {
        try {
            const remainingMs = this.getTimeRemainingMs();
            if (remainingMs < MIN_VALIDITY_SECONDS * 1000) {
                await this.sleep(remainingMs + BOUNDARY_SLACK_MS);
            }

            const currentWindow = Math.floor(
                Date.now() / (TOTP_STEP_SECONDS * 1000)
            );
            return speakeasy.totp({
                secret: this.seed,
                encoding: 'base32',
                time: currentWindow * TOTP_STEP_SECONDS,
                step: TOTP_STEP_SECONDS
            });
        } catch (error) {
            throw new Error(`Failed to generate TOTP: ${error.message}`);
        }
    }

    getTimeRemainingMs() {
        return TOTP_STEP_SECONDS * 1000 - (Date.now() % (TOTP_STEP_SECONDS * 1000));
    }

    /** Fractional seconds remaining in the current TOTP window. */
    getTimeRemaining() {
        return this.getTimeRemainingMs() / 1000;
    }

    sleep(milliseconds) {
        return new Promise((resolve) => setTimeout(resolve, milliseconds));
    }
}

async function main() {
    const totpGen = new TotpGenerator(process.argv[2]);
    console.log(await totpGen.generateTotp());
}
// Only run the CLI entry point when invoked directly (`node totpGenerator.js`); requiring
// this file as a module (e.g. from provideBrowserCredentials.js) must not construct a
// TotpGenerator with no seed, which throws and crashes the whole process as an unhandled
// rejection under Node 20's default --unhandled-rejections=throw.
if (require.main === module) {
    main().catch((error) => {
        console.error(error.message);
        process.exit(1);
    });
}

module.exports = TotpGenerator;
