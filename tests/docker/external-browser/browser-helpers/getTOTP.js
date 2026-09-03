const TotpGenerator = require('./totpGenerator');

async function main() {
    try {
        const seed = process.argv[2] || process.env.SNOWFLAKE_AUTH_MFA_SEED;
        
        if (!seed) {
            console.error('Usage: node getTOTP.js <seed>');
            console.error('Or set SNOWFLAKE_AUTH_MFA_SEED environment variable');
            process.exit(1);
        }

        const totpGen = new TotpGenerator(seed);
        // generateTotp returns the bare token and does not print. CLI stdout
        // must stay a single 6-digit line for whitespace-parsing callers.
        console.log(await totpGen.generateTotp());
        process.exit(0);
        
    } catch (error) {
        console.error('Error:', error.message);
        process.exit(1);
    }
}

main();
