const speakeasy = require('speakeasy');

class TotpGenerator {
    constructor(seed) {
        this.seed = seed || process.env.SNOWFLAKE_AUTH_MFA_SEED;
        
        if (!this.seed) {
            throw new Error('TOTP seed must be provided as parameter or set in SNOWFLAKE_AUTH_MFA_SEED environment variable');
        }
    }
    
    /**
     * Generate a TOTP code based on the current time.
     * If less than 5 seconds remaining, returns current + future tokens.
     * Otherwise returns past + current + future tokens.
     */
        async generateTotp() {
        try {
            const currentWindow = Math.floor(Date.now() / 30000);
            const nextWindow = currentWindow + 1;
            const timeRemaining = this.getTimeRemaining();
            
            const pastToken = speakeasy.totp({
                secret: this.seed,
                encoding: 'base32',
                time: (currentWindow - 1) * 30,
                step: 30,
                window: 1
            });
            
            const currentToken = speakeasy.totp({
                secret: this.seed,
                encoding: 'base32',
                time: currentWindow * 30,
                step: 30,
                window: 1
            });
            
            const futureToken = speakeasy.totp({
                secret: this.seed,
                encoding: 'base32',
                time: nextWindow * 30,
                step: 30,
                window: 1
            });
            
            if (timeRemaining < 5) {
                const tokens = { current: currentToken, future: futureToken };
                console.log(`${currentToken} ${futureToken}`);
                return tokens;
            } else {
                const tokens = { past: pastToken, current: currentToken, future: futureToken };
                console.log(`${pastToken} ${currentToken} ${futureToken}`);
                return tokens;
            }
        } catch (error) {
            throw new Error(`Failed to generate TOTP: ${error.message}`);
        }
    }
    
    /**
     * Return the next timing window's token immediately.
     */
        async refreshToken() {
        try {
            const currentWindow = Math.floor(Date.now() / 30000);
            const futureWindow = currentWindow + 1;
            
            const futureToken = speakeasy.totp({
                secret: this.seed,
                encoding: 'base32',
                time: futureWindow * 30,
                step: 30,
                window: 1
            });
            
            console.log(futureToken);
            return futureToken;
            
        } catch (error) {
            throw new Error(`Failed to refresh TOTP: ${error.message}`);
        }
    }

    verifyTotp(token) {
        try {
            const isValid = speakeasy.totp.verify({
                secret: this.seed,
                encoding: 'base32',
                token: token,
                step: 30,
                window: 2
            });
            
            console.log(`TOTP verification for ${token}: ${isValid}`);
            return isValid;
        } catch (error) {
            console.error(`Failed to verify TOTP: ${error.message}`);
            return false;
        }
    }
    
    getTimeRemaining() {
        const step = 30;
        const currentTime = Math.floor(Date.now() / 1000);
        const timeRemaining = step - (currentTime % step);
        return timeRemaining;
    }
}

async function main() {
    const totpGen = new TotpGenerator();
    const code = await totpGen.generateTotp();
    return code;
}
main();

module.exports = TotpGenerator;
