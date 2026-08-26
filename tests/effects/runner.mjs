/**
 * VeilAnon — Effects System Test Runner
 * Executes effects.test.mjs and plugin.test.mjs, reports results.
 *
 * Usage: node tests/effects/runner.mjs
 */

import { runEffectsTests } from './effects.test.mjs';
import { runPluginTests } from './plugin.test.mjs';

async function main() {
  console.log('\x1b[1m\x1b[35m' + '═'.repeat(60) + '\x1b[0m');
  console.log('\x1b[1m\x1b[35m  VEILANON — EFFECTS SYSTEM UNIT TESTS\x1b[0m');
  console.log('\x1b[1m\x1b[35m' + '═'.repeat(60) + '\x1b[0m');

  const startTime = Date.now();
  let totalPassed = 0;
  let totalFailed = 0;
  const allFailures = [];

  try {
    // Run effects tests
    const effectsResult = await runEffectsTests();
    totalPassed += effectsResult.passed;
    totalFailed += effectsResult.failed;
    allFailures.push(...effectsResult.failures);

    // Run plugin tests
    const pluginResult = await runPluginTests();
    totalPassed += pluginResult.passed;
    totalFailed += pluginResult.failed;
    allFailures.push(...pluginResult.failures);
  } catch (err) {
    console.error('\x1b[31mFatal error during test execution:\x1b[0m', err);
    process.exit(1);
  }

  const duration = ((Date.now() - startTime) / 1000).toFixed(2);
  const total = totalPassed + totalFailed;
  const passRate = total > 0 ? ((totalPassed / total) * 100).toFixed(1) : '0.0';

  // ── Summary ──────────────────────────────────────────────────────────────
  console.log('\n' + '═'.repeat(60));
  console.log('\x1b[1m\x1b[35m  EFFECTS SYSTEM — TEST EXECUTION SUMMARY\x1b[0m');
  console.log('═'.repeat(60));
  console.log(`\x1b[1mTOTAL TESTS:\x1b[0m  \x1b[1m${total}\x1b[0m`);
  console.log(`\x1b[1mPASSED:\x1b[0m       \x1b[32m\x1b[1m${totalPassed}\x1b[0m`);
  console.log(`\x1b[1mFAILED:\x1b[0m       ${totalFailed > 0 ? `\x1b[31m\x1b[1m${totalFailed}\x1b[0m` : '\x1b[90m0\x1b[0m'}`);
  console.log(`\x1b[1mPASS RATE:\x1b[0m    \x1b[1m${passRate}%\x1b[0m`);
  console.log(`\x1b[1mTOTAL TIME:\x1b[0m   \x1b[1m${duration}s\x1b[0m`);
  console.log('─'.repeat(60));

  if (allFailures.length > 0) {
    console.log('\n\x1b[1mFAILED TESTS:\x1b[0m');
    for (const f of allFailures) {
      console.log(`  \x1b[31m- ${f.name}\x1b[0m`);
      console.log(`    ${f.error}`);
    }
  }

  if (totalFailed === 0 && total > 0) {
    console.log(`\n\x1b[1m\x1b[42m\x1b[30m  ALL ${total} EFFECTS SYSTEM TESTS PASSED  \x1b[0m\n`);
    process.exit(0);
  } else {
    console.log(`\n\x1b[1m\x1b[41m\x1b[37m  EFFECTS SYSTEM TESTS FAILED  \x1b[0m\n`);
    process.exit(1);
  }
}

main();
