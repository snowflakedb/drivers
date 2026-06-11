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
        const code = await totpGen.generateTotp();
        
        console.log(code);
        process.exit(0);
        
    } catch (error) {
        console.error('Error:', error.message);
        process.exit(1);
    }
}

main();
