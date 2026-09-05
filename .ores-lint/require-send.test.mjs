import assert from 'node:assert/strict';

import { analyzeSource } from './require-send.mjs';

const findingCount = (source, language) => analyzeSource(source, language).length;

const cases = [
  {
    name: 'reports a discarded Rust event',
    language: 'rust',
    source: 'fn emit(logger: &Logger) { logger.info("ready"); }',
    expected: 1,
  },
  {
    name: 'accepts a delivered Rust event',
    language: 'rust',
    source: 'fn emit(logger: &Logger) { logger.info("ready").send(); }',
    expected: 0,
  },
  {
    name: 'tracks and clears an assigned Rust event',
    language: 'rust',
    source: 'fn emit(logger: &Logger) { let event = logger.info("ready"); event.send(); }',
    expected: 0,
  },
  {
    name: 'reports an assigned Rust event that leaves scope unsent',
    language: 'rust',
    source: 'fn emit(logger: &Logger) { let event = logger.info("ready"); consume(); }',
    expected: 1,
  },
  {
    name: 'accepts a returned Rust event for caller delivery',
    language: 'rust',
    source: 'fn event(logger: &Logger) -> Event { logger.info("ready") }',
    expected: 0,
  },
  {
    name: 'honors the cross-language next-line suppression',
    language: 'dart',
    source: '// ores-lint-disable-next-line require-send\nlogger.info("ready");',
    expected: 0,
  },
  {
    name: 'reports a discarded Dart event',
    language: 'dart',
    source: 'void emit(Logger logger) { logger.warn("offline"); }',
    expected: 1,
  },
  {
    name: 'accepts the Dart convenience logger method',
    language: 'dart',
    source: 'void emit(Logger logger) { logger.log(level, "offline", context, fields); }',
    expected: 0,
  },
  {
    name: 'reports an assigned Gleam event that leaves scope unsent',
    language: 'gleam',
    source: 'pub fn emit(logger) { let event = logging.info(logger, "ready")\nNil }',
    expected: 1,
  },
  {
    name: 'accepts a delivered Gleam pipeline',
    language: 'gleam',
    source: 'pub fn emit(logger) { logging.info(logger, "ready") |> logging.send }',
    expected: 0,
  },
];

for (const testCase of cases) {
  assert.equal(
    findingCount(testCase.source, testCase.language),
    testCase.expected,
    testCase.name,
  );
}

process.stdout.write(`require-send fixtures passed (${cases.length})\n`);
