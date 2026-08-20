/**
 * VeilAnon v0.0.1 — E2E Test Suite Runner
 * Standalone test runner that executes all 4 tiers, records metrics,
 * formats structured console reporting, and exits with code 0 on 100% pass.
 */

import { runTier1Tests } from './tier1-feature-coverage.test.mjs';
import { runTier2Tests } from './tier2-boundary-corner.test.mjs';
import { runTier3Tests } from './tier3-pairwise-combinations.test.mjs';
import { runTier4Tests } from './tier4-application-scenarios.test.mjs';
import { TIERS, FEATURES } from './harness/types.mjs';

class TestReporter {
  constructor() {
    this.currentTier = null;
    this.tierResults = new Map(); // tierId -> { tier, tests: [], passed: 0, failed: 0, startTime, duration: 0 }
    this.featureResults = new Map(); // featureId -> { feature, count: 0, passed: 0 }
    this.startTime = Date.now();
    this.totalPassed = 0;
    this.totalFailed = 0;
  }

  startTier(tier) {
    this.currentTier = tier;
    this.tierResults.set(tier.id, {
      tier,
      tests: [],
      passed: 0,
      failed: 0,
      startTime: Date.now(),
      duration: 0,
    });
    console.log(`\n\x1b[1m\x1b[36m▶ Running ${tier.name} (Min Target: ${tier.minTests} tests)...\x1b[0m`);
  }

  async test(featureOrGroup, testName, testFn) {
    const tierStat = this.tierResults.get(this.currentTier.id);
    const t0 = performance.now();
    let passed = false;
    let error = null;

    try {
      await testFn();
      passed = true;
      tierStat.passed++;
      this.totalPassed++;
      const duration = (performance.now() - t0).toFixed(1);
      console.log(`  \x1b[32m✔\x1b[0m \x1b[90m[${tierStat.tests.length + 1}]\x1b[0m ${testName} \x1b[90m(${duration}ms)\x1b[0m`);
    } catch (err) {
      passed = false;
      error = err;
      tierStat.failed++;
      this.totalFailed++;
      const duration = (performance.now() - t0).toFixed(1);
      console.log(`  \x1b[31m✖\x1b[0m \x1b[90m[${tierStat.tests.length + 1}]\x1b[0m ${testName} \x1b[90m(${duration}ms)\x1b[0m`);
      console.log(`    \x1b[31mError: ${err.message}\x1b[0m`);
      if (err.actual !== undefined && err.expected !== undefined) {
        console.log(`    \x1b[90mExpected: ${JSON.stringify(err.expected)}\x1b[0m`);
        console.log(`    \x1b[90mActual:   ${JSON.stringify(err.actual)}\x1b[0m`);
      }
      if (err.stack) {
        console.log(`    \x1b[90m${err.stack.split('\n').slice(1, 4).join('\n    ')}\x1b[0m`);
      }
    }

    tierStat.tests.push({ name: testName, feature: featureOrGroup, passed, error });

    if (featureOrGroup && featureOrGroup.id) {
      const fId = featureOrGroup.id;
      if (!this.featureResults.has(fId)) {
        this.featureResults.set(fId, { feature: featureOrGroup, count: 0, passed: 0 });
      }
      const fStat = this.featureResults.get(fId);
      fStat.count++;
      if (passed) fStat.passed++;
    }
  }

  printSummary() {
    const totalDuration = ((Date.now() - this.startTime) / 1000).toFixed(2);
    const totalTests = this.totalPassed + this.totalFailed;
    const passRate = totalTests > 0 ? ((this.totalPassed / totalTests) * 100).toFixed(1) : '0.0';

    console.log('\n' + '═'.repeat(78));
    console.log(`\x1b[1m\x1b[35m  VEILANON v0.0.1 — E2E TEST SUITE EXECUTION SUMMARY\x1b[0m`);
    console.log('═'.repeat(78));

    console.log('\n\x1b[1mTIER BREAKDOWN:\x1b[0m');
    console.log('┌──────┬──────────────────────────────────────────────┬────────┬────────┬───────────┬────────┐');
    console.log('│ Tier │ Name                                         │ Passed │ Failed │ Target    │ Status │');
    console.log('├──────┼──────────────────────────────────────────────┼────────┼────────┼───────────┼────────┤');

    let allTiersMet = true;
    for (const [tierId, stat] of this.tierResults.entries()) {
      const met = stat.passed >= stat.tier.minTests && stat.failed === 0;
      if (!met) allTiersMet = false;
      const statusStr = met ? '\x1b[32mPASS\x1b[0m  ' : '\x1b[31mFAIL\x1b[0m  ';
      const tierIdStr = `T${tierId}`.padEnd(4);
      const nameStr = stat.tier.name.padEnd(44);
      const passStr = String(stat.passed).padStart(6);
      const failStr = String(stat.failed).padStart(6);
      const targetStr = `>= ${stat.tier.minTests}`.padEnd(9);
      console.log(`│ ${tierIdStr} │ ${nameStr} │ ${passStr} │ ${failStr} │ ${targetStr} │ ${statusStr} │`);
    }
    console.log('└──────┴──────────────────────────────────────────────┴────────┴────────┴───────────┴────────┘');

    console.log('\n\x1b[1mFEATURE COVERAGE (15 Features from PROJECT.md):\x1b[0m');
    for (const [fKey, feat] of Object.entries(FEATURES)) {
      const fStat = this.featureResults.get(feat.id);
      const count = fStat ? fStat.count : 0;
      const passed = fStat ? fStat.passed : 0;
      const icon = (count >= 10 && passed === count) ? '\x1b[32m✔\x1b[0m' : '\x1b[33m▲\x1b[0m';
      console.log(`  ${icon} Feature ${String(feat.id).padStart(2)}: ${feat.name.padEnd(42)} -> \x1b[1m${passed}/${count} tests\x1b[0m`);
    }

    console.log('\n' + '─'.repeat(78));
    console.log(`\x1b[1mTOTAL TESTS:\x1b[0m    \x1b[1m${totalTests}\x1b[0m (Target: >= 173)`);
    console.log(`\x1b[1mPASSED:\x1b[0m         \x1b[32m\x1b[1m${this.totalPassed}\x1b[0m`);
    console.log(`\x1b[1mFAILED:\x1b[0m         ${this.totalFailed > 0 ? `\x1b[31m\x1b[1m${this.totalFailed}\x1b[0m` : '\x1b[90m0\x1b[0m'}`);
    console.log(`\x1b[1mPASS RATE:\x1b[0m      \x1b[1m${passRate}%\x1b[0m`);
    console.log(`\x1b[1mTOTAL TIME:\x1b[0m     \x1b[1m${totalDuration}s\x1b[0m`);
    console.log('─'.repeat(78));

    const totalTargetMet = totalTests >= 173 && this.totalFailed === 0 && allTiersMet;
    if (totalTargetMet) {
      console.log(`\n\x1b[1m\x1b[42m\x1b[30m  ALL 173+ E2E TESTS PASSED WITH 100% VERIFICATION INTEGRITY  \x1b[0m\n`);
      return 0;
    } else {
      console.log(`\n\x1b[1m\x1b[41m\x1b[37m  E2E TEST SUITE FAILED OR CRITERIA NOT MET  \x1b[0m\n`);
      return 1;
    }
  }
}

async function main() {
  console.log('\x1b[1m\x1b[35m' + '═'.repeat(78) + '\x1b[0m');
  console.log('\x1b[1m\x1b[35m  VEILANON v0.0.1 — OPAQUE-BOX E2E TEST RUNNER\x1b[0m');
  console.log('\x1b[1m\x1b[35m' + '═'.repeat(78) + '\x1b[0m');

  const reporter = new TestReporter();

  try {
    await runTier1Tests(reporter);
    await runTier2Tests(reporter);
    await runTier3Tests(reporter);
    await runTier4Tests(reporter);
  } catch (fatalErr) {
    console.error('\x1b[31mFatal unhandled error during test suite run:\x1b[0m', fatalErr);
    process.exit(1);
  }

  const exitCode = reporter.printSummary();
  process.exit(exitCode);
}

main();
