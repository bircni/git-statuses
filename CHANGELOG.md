# Changelog - [git-statuses](https://github.com/bircni/git-statuses)

All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

## [0.5.1](https://github.com/bircni/git-statuses/compare/0.5.0..0.5.1) - 2025-07-21

### Bug Fixes

- remove unused crates - ([7a0831d](https://github.com/bircni/git-statuses/commit/7a0831d1f90131a68cbc23264b4cff33b1e804b4)) - Nicolas

### Documentation

- update README documentation - ([ebfa500](https://github.com/bircni/git-statuses/commit/ebfa5008495b82cfa28805334f18895dba1bf780)) - Nicolas

### Features

- **(interactive)** add an interactive mode for repos ([#30](https://github.com/bircni/git-statuses/issues/30)) - ([4713543](https://github.com/bircni/git-statuses/commit/47135430888ba26f54cf11c4056cb642b9830346)) - Nicolas

### Tests

- fix errors - ([b5e97b5](https://github.com/bircni/git-statuses/commit/b5e97b5a50b5c0a038c37b95e50eb743a801e23f)) - Nicolas

## [0.5.0](https://github.com/bircni/git-statuses/compare/0.4.1..0.5.0) - 2025-07-14

### Features

- add Unpushed Status - ([b0171ac](https://github.com/bircni/git-statuses/commit/b0171ac715b26b30a3069cc482d4d18bf551c8df)) - Nicolas
- Implement stash detection and local-only branch indicators - ([8ae2655](https://github.com/bircni/git-statuses/commit/8ae2655edda39cad056747790af22907be79b56c)) - Nicolas

### Tests

- Add comprehensive test coverage for critical paths and edge cases - ([c9bc0ea](https://github.com/bircni/git-statuses/commit/c9bc0eae4ca42176041f84d0900aabc6a462e3a6)) - Nicolas

## [0.4.1](https://github.com/bircni/git-statuses/compare/0.4.0..0.4.1) - 2025-07-10

### Features

- add the ability to generate shell completions - ([c370b5e](https://github.com/bircni/git-statuses/commit/c370b5e0c5fd2696aaa84fa3b6a9e8d5ef6a0995)) - Fotis Gimian
- add option to only show non clean repositories - ([9c989c0](https://github.com/bircni/git-statuses/commit/9c989c0b8b3b94549339b5256cdc28d7b6b8f50a)) - Nicolas
- add option to generate a report directly from a repository - ([a8406c3](https://github.com/bircni/git-statuses/commit/a8406c3acebd45aad8d42244495175dfd00cfa97)) - Nicolas
- add option to show the path to the repository - ([3993a7e](https://github.com/bircni/git-statuses/commit/3993a7eb7d5d549a94d32efe60c81a4cc05cf81e)) - Nicolas

## [0.4.0](https://github.com/bircni/git-statuses/compare/0.3.1..0.4.0) - 2025-07-08

### Documentation

- update README with new command-line options and improve example image - ([66ad81a](https://github.com/bircni/git-statuses/commit/66ad81a737652b95a2fad6095a57cecf3501d7d8)) - Nicolas

### Features

- add the ability to use a condensed layout ([#10](https://github.com/bircni/git-statuses/issues/10)) - ([1c626ca](https://github.com/bircni/git-statuses/commit/1c626ca0b423b45ffbe6db05df4fb630f1f3d843)) - Nicolas
- add subdir option to CLI and RepoInfo, enhance repository path handling - ([ffaa6fb](https://github.com/bircni/git-statuses/commit/ffaa6fbbfdff468a11c49c5c5ff0aba9f7f67d27)) - Nicolas
- implement RepoStatus handling, enhance repository status display - ([9de728d](https://github.com/bircni/git-statuses/commit/9de728d99d2f0fc76d115d0a632d9e2bdf27239b)) - Nicolas

### Tests

- set git config ([#8](https://github.com/bircni/git-statuses/issues/8)) - ([e0b03ca](https://github.com/bircni/git-statuses/commit/e0b03ca9677759b803970280c8840a44da0df8d8)) - Guilhem Saurel
- enhance gitinfo tests for invalid HEAD and add printer tests - ([ae99e66](https://github.com/bircni/git-statuses/commit/ae99e66846752dcf7f28a71b13e360bd6d2e57d1)) - Nicolas

## [0.3.1](https://github.com/bircni/git-statuses/compare/0.3.0..0.3.1) - 2025-07-06

### Bug Fixes

- depth is not a bool ([#5](https://github.com/bircni/git-statuses/issues/5)) - ([63f8d96](https://github.com/bircni/git-statuses/commit/63f8d9625feab551de4f7baf3e327b06f79b219f)) - Guilhem Saurel

### Documentation

- update minimal rust version in README ([#6](https://github.com/bircni/git-statuses/issues/6)) - ([0789d45](https://github.com/bircni/git-statuses/commit/0789d450554b7d5f30a38033f0b3a1640a68929d)) - Guilhem Saurel

## [0.3.0](https://github.com/bircni/git-statuses/compare/0.2.1..0.3.0) - 2025-07-05

### Bug Fixes

- fetch option to not destroy the whole process on failure - ([fe2c166](https://github.com/bircni/git-statuses/commit/fe2c166f74ccdb20bae0f8e146750017bcfe7f30)) - Nicolas
- update summary function to include failed repositories count - ([16a7f1e](https://github.com/bircni/git-statuses/commit/16a7f1ef8d7c4649154b1fbc78b094d60c57e307)) - Nicolas

### Documentation

- enhance documentation for RepoInfo constructor and utility functions - ([ae9f440](https://github.com/bircni/git-statuses/commit/ae9f44095dca59531abfa9bdc4257236acc026d9)) - Nicolas

### Features

- add a legend to explain the styles & enhance the printing output styling - ([5e1036d](https://github.com/bircni/git-statuses/commit/5e1036dd306066fad8ac5ab863ba01994935a985)) - Nicolas

## [0.2.1](https://github.com/bircni/git-statuses/compare/0.2.0..0.2.1) - 2025-07-05

### Bug Fixes

- repository url was set to a wrong repo - ([68a02da](https://github.com/bircni/git-statuses/commit/68a02da391d0edc42fcd81eaca3204c137b03cc3)) - Nicolas
- README example image - ([a5d4da9](https://github.com/bircni/git-statuses/commit/a5d4da9c41842d81a232abe94ce99c6d4bb8a745)) - Nicolas

## [0.2.0](https://github.com/bircni/git-statuses/compare/0.1.0..0.2.0) - 2025-07-05

### Bug Fixes

- update fetch_origin documentation to English - ([6dffc1c](https://github.com/bircni/git-statuses/commit/6dffc1c7e1829a7d68ea527ea89915f34ad421d9)) - Nicolas

### Documentation

- add image as example - ([2368975](https://github.com/bircni/git-statuses/commit/2368975b09f133c18299b09137e21147c709f2c2)) - Nicolas

### Features

- add fetch option to CLI - ([ab89ee2](https://github.com/bircni/git-statuses/commit/ab89ee2247483002cbf137cca5f3c94835fa6941)) - Nicolas

### Linting

- collapse if let Statements (Rust > 1.88) - ([8bc32d1](https://github.com/bircni/git-statuses/commit/8bc32d1bd47d2a9e48f1408a9137213bae925912)) - Nicolas

### Tests

- update test-files to match new cli options - ([1e31aed](https://github.com/bircni/git-statuses/commit/1e31aed7984ab8a2a9118138d6a1511e060e1e30)) - Nicolas

### Build

- enable correct deployments - ([462c9ae](https://github.com/bircni/git-statuses/commit/462c9ae71c957b72ec276a45a0f84cb653c7b047)) - Nicolas

## [0.1.0](https://github.com/bircni/git-statuses/compare/0.0.1..0.1.0) - 2025-07-04

### Features

- enhancements - ([c780e10](https://github.com/bircni/git-statuses/commit/c780e1031ef1c0f577f46a2f2470e1e91e0412ca)) - Nicolas

## [0.0.1] - 2025-07-04
<!-- generated by git-cliff -->
