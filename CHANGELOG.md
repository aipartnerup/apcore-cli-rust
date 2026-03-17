# Changelog

All notable changes to this project will be documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

## [0.2.0] - 2026-03-17

### Added
- Initial project scaffold ported from apcore-cli-python 0.2.0
- `src/` module stubs: cli, config, discovery, output, approval, ref_resolver,
  schema_parser, shell, _sandbox_runner, security/{audit,auth,config_encryptor,sandbox}
- Integration test stubs in `tests/` (TDD red phase)
- Example extension modules: math/{add,multiply}, text/{upper,reverse,wordcount},
  sysutil/{info,disk,env}
- `examples/run_examples.sh` demo script
