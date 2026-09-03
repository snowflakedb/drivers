const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const test = require('node:test');
const speakeasy = require('speakeasy');
const TotpGenerator = require('./totpGenerator.js');

const SEED = 'JBSWY3DPEHPK3PXP';

test('generateTotp emits only the currently valid code', async () => {
    const originalNow = Date.now;
    const originalLog = console.log;
    let now = 45_000;
    const output = [];

    Date.now = () => now;
    console.log = (value) => output.push(value);

    try {
        const generator = new TotpGenerator(SEED);
        const result = await generator.generateTotp();
        const expected = speakeasy.totp({
            secret: SEED,
            encoding: 'base32',
            time: 30,
            step: 30
        });

        assert.equal(typeof result, 'string');
        assert.equal(result.current, undefined);
        assert.equal(result, expected);
        assert.deepEqual(output, []);
    } finally {
        Date.now = originalNow;
        console.log = originalLog;
    }
});

test('generateTotp waits rather than returning an adjacent-window code', async () => {
    const originalNow = Date.now;
    const originalLog = console.log;
    let now = 59_000;
    const output = [];
    let sleptMs = 0;

    Date.now = () => now;
    console.log = (value) => output.push(value);

    try {
        const generator = new TotpGenerator(SEED);
        generator.sleep = async (milliseconds) => {
            sleptMs = milliseconds;
            now += milliseconds;
        };

        const result = await generator.generateTotp();
        const expected = speakeasy.totp({
            secret: SEED,
            encoding: 'base32',
            time: 60,
            step: 30
        });

        assert.equal(typeof result, 'string');
        assert.equal(result.current, undefined);
        assert.equal(result, expected);
        assert.deepEqual(output, []);
        assert.ok(
            sleptMs >= 1000 && sleptMs <= 1400,
            `expected ~1200ms boundary wait, got ${sleptMs}`
        );
    } finally {
        Date.now = originalNow;
        console.log = originalLog;
    }
});

test('CLI stdout is a single 6-digit token', () => {
    const result = spawnSync(process.execPath, [require.resolve('./totpGenerator.js'), SEED], {
        encoding: 'utf8',
        timeout: 15_000
    });
    assert.equal(result.status, 0, result.stderr);
    const line = result.stdout.trim();
    assert.match(line, /^\d{6}$/);
    assert.equal(line.includes('{'), false);
});
