# Changelog

All notable changes to tld are documented here.

## [0.1.12] - 2026-04-19

### Bug Fixes

- Suppress verbosity of add and connect commands ([`cdbede7`](https://github.com/Mertcikla/tld-cli/commit/cdbede7ee4898552b23621bcbf733a35a61000b5))

### Documentation

- Document element tags and usage; raise internal element rule to 15 ([`f1dfc6a`](https://github.com/Mertcikla/tld-cli/commit/f1dfc6a9b18f4a96c2899c0aecf69238badebeb5))
- Raise view density to 15 elements and add tag guidance ([`3ada4dc`](https://github.com/Mertcikla/tld-cli/commit/3ada4dc6d736ccbe586b5f89e724cc235e4df041))

### Features

- Add code coverage workflow and update documentation for coverage commands ([`e82687f`](https://github.com/Mertcikla/tld-cli/commit/e82687f1c25c7b675b82ff073a8ffa40b130814f))
- Enhance workspace management with new metadata handling and tests ([`5b83ddd`](https://github.com/Mertcikla/tld-cli/commit/5b83ddd97f1d896c07a072b4bac75d6c763529fd))
- Update dependencies and improve CLI command usage in documentation ([`3adff46`](https://github.com/Mertcikla/tld-cli/commit/3adff460515b4224e6ab6946bbc8bfc78c24bbb9))
- Implement repository root management and enhance CLI commands ([`8d2037b`](https://github.com/Mertcikla/tld-cli/commit/8d2037b3936b2181c7eee01bb223a50b25a37449))
- Add --ref flag to 'tld add' and update docs and tests ([`3bc3ae9`](https://github.com/Mertcikla/tld-cli/commit/3bc3ae948ced838f3e63a99b80ecbd6949a13902))
- Render plan and write report to --output; update docs and tests ([`2994bac`](https://github.com/Mertcikla/tld-cli/commit/2994bac46fe49eac89aa0a617fd1bceb9d2c88de))

### Refactoring

- Update test codebase and workflows while removing deprecated skill-writer ([`a3de165`](https://github.com/Mertcikla/tld-cli/commit/a3de16566d34ebc0dace8286156848c5515c3c3a))
- Update CLI modules and expand test codebase with new project configurations and artifacts ([`66a9395`](https://github.com/Mertcikla/tld-cli/commit/66a9395ad19d06e335df8d21be8dd0bfc10e0e60))

## [0.1.11] - 2026-04-18

### Features

- Add auto-collapse projection and --max-elements cap ([`f12e950`](https://github.com/Mertcikla/tld-cli/commit/f12e95099b718ce3770b39efcc5fac0674457882))

## [0.1.10] - 2026-04-17

### Documentation

- Update installation instructions and add tag management features ([`6232bf9`](https://github.com/Mertcikla/tld-cli/commit/6232bf9ef829d943b670596a7c6919423f545cfa))

### Features

- Add .venv, build, and out to init ignore list ([`9a92662`](https://github.com/Mertcikla/tld-cli/commit/9a92662f055f4496062513fb789fbc8cb88ac79b))
- Merge near-duplicate auto-generated tags and rename pruning fn ([`c97dcb5`](https://github.com/Mertcikla/tld-cli/commit/c97dcb528c0f95587018d1dd73d45d84e023b8d9))
- Add build.rs to set TLD_BUILD_VERSION and show it in CLI ([`dbe3060`](https://github.com/Mertcikla/tld-cli/commit/dbe306049113ff3a57e4aea6c6a52eb0f010eb67))
- Prune sparse auto-generated tags in projections ([`70a1ff4`](https://github.com/Mertcikla/tld-cli/commit/70a1ff4fe9d4b7f03fbb00cfd26621a069855edc))
- Add tagging for scanned elements ([`fba2abf`](https://github.com/Mertcikla/tld-cli/commit/fba2abf4fae359cbfaccab725524ed1971fac373))
- Add HTTP endpoint recognition from annotations ([`a4fa9ec`](https://github.com/Mertcikla/tld-cli/commit/a4fa9ecfcb6371c2a8a95b1810141e1d7ae5c9c7))
- Add repository URL to BuildContext and update projections to include it ([`f4e6db7`](https://github.com/Mertcikla/tld-cli/commit/f4e6db7d4692d6094d37625df6f3debe146aeb40))
- Enhance view handling in plan generation and add tests for view auto-enablement ([`a9a20b4`](https://github.com/Mertcikla/tld-cli/commit/a9a20b422ad3251e01c05689701f40a1325d38b3))
- Add install_cmd for LSPs and prompt to install missing servers ([`9f44048`](https://github.com/Mertcikla/tld-cli/commit/9f440486440ef5436b560a4a26cdb8f29d96fbf9))
- Add benchmarking script for `tld analyze` and update `.gitignore` and `Makefile` ([`a21d053`](https://github.com/Mertcikla/tld-cli/commit/a21d053a4b4cbfc7d19734d91b5169e938e0fd25))
- Add language-specific query files for C++, Go, Java, JavaScript, Python, Rust, and TypeScript ([`dae3891`](https://github.com/Mertcikla/tld-cli/commit/dae389185f5c48d4700f337ecee440006bdadbe4))
- Enhance config loading with environment variable support and add tests for config functions ([`6caf4a7`](https://github.com/Mertcikla/tld-cli/commit/6caf4a75604ac8deecde96b202dab90b87a83dc7))
- Improve gRPC channel connection handling and add tests for URL normalization ([`7191584`](https://github.com/Mertcikla/tld-cli/commit/7191584bfc7710ce9ac59a9d4a9eef033f35d283))
- Enhance config loading with environment variable support for server URL, API key, and organization ID ([`38590e3`](https://github.com/Mertcikla/tld-cli/commit/38590e36c12832a77d64fd267b85eb2e96d1a5b9))
- Add new flags to analyze command for enhanced functionality ([`a82fff6`](https://github.com/Mertcikla/tld-cli/commit/a82fff67292f8e20c06d6ab654f0f2cf37eea1fc))
- Enhance analyze command with new flags and remove deprecated lsp flag ([`bf7ba34`](https://github.com/Mertcikla/tld-cli/commit/bf7ba34dd105f8d415e633faca50b1b36a11872e))
- Enhance salience scoring with cyclomatic and cognitive complexity metrics ([`4fe9a53`](https://github.com/Mertcikla/tld-cli/commit/4fe9a534f3f23d3163f0ea5efd0a24eac232b0e3))
- Introduce semantic analysis types and syntax extraction ([`274f023`](https://github.com/Mertcikla/tld-cli/commit/274f02349a549769fa34fcd9fead2385784311dc))
- Update language parsers and expand test-codebase configuration files ([`69ffc30`](https://github.com/Mertcikla/tld-cli/commit/69ffc301e05b3c081104ab70646550463f3f659c))
- Initialize test-codebase repository and update multi-language project configurations ([`84eaed6`](https://github.com/Mertcikla/tld-cli/commit/84eaed64d86c61b7bdc46b2862323a82c63a15b9))
- Add initial parsers and implement codebase analysis tests ([`487b09d`](https://github.com/Mertcikla/tld-cli/commit/487b09d56f79358c0861669bf35871b8600201db))
- Add icons configuration and update workspace analysis and conversion logic ([`64b69a6`](https://github.com/Mertcikla/tld-cli/commit/64b69a6b29dea0bbd03862634281539be0ee4a41))

### Refactoring

- Simplify is_git_repo result handling using is_ok_and ([`b1d0081`](https://github.com/Mertcikla/tld-cli/commit/b1d0081d1bdceb2d3a837f70aec6694d3e7cd9fd))
- Analyzer - pass AutoTagOptions by value and add PathContext ([`a5a485d`](https://github.com/Mertcikla/tld-cli/commit/a5a485d24a9bf8674d797a5b9c54c3ad48bad4fd))
- Streamline code structure and improve readability across multiple modules ([`17c6cf9`](https://github.com/Mertcikla/tld-cli/commit/17c6cf9336fa46104d00f048e06530d3a51376d2))
- Update CLI and workspace logic while expanding test-codebase configuration and infrastructure ([`406ee24`](https://github.com/Mertcikla/tld-cli/commit/406ee24472aa7a679af927219e28ea43f59951a3))
- Update codebase analysis logic and initialize test repository infrastructure ([`a335eb7`](https://github.com/Mertcikla/tld-cli/commit/a335eb70649a94b5a1b4eb233bbff0df38a80454))
- Update test codebase structure and initialize git repositories for testing environments ([`ed75176`](https://github.com/Mertcikla/tld-cli/commit/ed75176be9b42125e543cf4c371a98b7a1f6b031))
- Update CLI logic and test codebase structure while initializing git hooks for examples ([`1b41b4f`](https://github.com/Mertcikla/tld-cli/commit/1b41b4faefb3717a9f32afb2ce76d4fa42fa83c0))

## [0.1.8] - 2026-04-15

### Bug Fixes

- Update references from diagrams to views and elements in conversion and removal commands ([`f7986e8`](https://github.com/Mertcikla/tld-cli/commit/f7986e8433130b2c2fd12b2c0f54f1dbe37439f7))

### Documentation

- Add CI and release instructions to CLAUDE.md ([`6150b09`](https://github.com/Mertcikla/tld-cli/commit/6150b094220ac370d85eb40a8c97aeb966b1d99c))
- Add skill-writer skill and update tld CLI and create-diagram docs ([`d67f99b`](https://github.com/Mertcikla/tld-cli/commit/d67f99b38a37a5415ae96ec1f9d42fe793c158a1))

### Features

- Integrate diag-proto crate and update CI instructions in CLAUDE.md ([`49a8966`](https://github.com/Mertcikla/tld-cli/commit/49a8966ce0d47a6f838ea5626634c45c1595eda3))
- Add Rust analyzer parser, --download option, and language docs ([`50b10e0`](https://github.com/Mertcikla/tld-cli/commit/50b10e02df7aabb175a0436ad606e04e6ed0ea25))
- Implement typescript analyzer and add test-codebase examples with configuration files ([`8ca1956`](https://github.com/Mertcikla/tld-cli/commit/8ca19566bb0100deb12ae3e41ad2d3cda89dd714))
- Use simpler dependency labels and set repository Technology ([`a8a8b9a`](https://github.com/Mertcikla/tld-cli/commit/a8a8b9a6788f816bb2b860f904803bb17f7296d3))
- Enhance Python import handling and update related tests to support view terminology ([`cdeee59`](https://github.com/Mertcikla/tld-cli/commit/cdeee5939e6b41153a8f7ffbd0f15cf89189e1fd))
- Implement support for multiple connector appends and enhance dependency labeling in analysis ([`f6f23ba`](https://github.com/Mertcikla/tld-cli/commit/f6f23ba54d5ee9f8002a74f14dbfcd3077f5bd28))
- Enhance symbol extraction to include cross-file and cross-folder references, add import handling ([`70f18a7`](https://github.com/Mertcikla/tld-cli/commit/70f18a79fd590e8ef97672c2f7321c2e377e2e24))
- Add test script to .gitignore ([`febd47c`](https://github.com/Mertcikla/tld-cli/commit/febd47c4823103d861004c4b9ea7bfe4a9308364))
- Remove legacy fields from ResourceCounts struct in LockFile ([`c2bd9c1`](https://github.com/Mertcikla/tld-cli/commit/c2bd9c14883baa3a5a6152d934d59d467aa5e8f8))
- Update .gitignore and remove empty configuration files ([`4448191`](https://github.com/Mertcikla/tld-cli/commit/44481915ebb7f752bf2ba0b7b539b3265fc5b51e))
- Refactor lock file management to use ResourceCounts struct for resource updates ([`fd67241`](https://github.com/Mertcikla/tld-cli/commit/fd67241ac65dbb7645dd5a7605310baa5b2e73bc))
- Add canonical view promotion for placement parents in Build function ([`751047e`](https://github.com/Mertcikla/tld-cli/commit/751047ed80cb47c9539da2202664fea28f2ece9c))
- Implement folder hierarchy creation in analyze command tests ([`83e1424`](https://github.com/Mertcikla/tld-cli/commit/83e142465f777fb29c10055458f5f7eae61ce942))
- Enhance lockfile management and metadata persistence ([`b670ba0`](https://github.com/Mertcikla/tld-cli/commit/b670ba044d81c2a558b16f25914884172eba12e7))
- Add support for multiple programming languages in analyzer ([`7d7c5b0`](https://github.com/Mertcikla/tld-cli/commit/7d7c5b0aba5889e01a2738af3c93f0dcfdfe1274))
- Initialize TLD configuration and connector files across multi-language example codebases ([`47888d0`](https://github.com/Mertcikla/tld-cli/commit/47888d024bd5c8406d3d5ad92fb263c00161293c))
- Initialize test-codebase repository and add TLD configuration files across multiple language projects ([`970127f`](https://github.com/Mertcikla/tld-cli/commit/970127fe257c0f5d5e3983c9e2189900ca0f80e0))
- Add symbol end_line and parent and resolve refs by file+line ([`7835468`](https://github.com/Mertcikla/tld-cli/commit/78354681ce67610b487c115aa78b1f0b38b2ccd0))
- Update element and connector commands to support field updates ([`f373d0b`](https://github.com/Mertcikla/tld-cli/commit/f373d0b149f1dbd2932ef341bace15c23fc0b779))
- Add validation options and default level; skip symbol checks ([`f75c9f4`](https://github.com/Mertcikla/tld-cli/commit/f75c9f45a06afb92de0cc2b4c450aa4855027bab))
- Update tests for elements handling ([`0ee9eb9`](https://github.com/Mertcikla/tld-cli/commit/0ee9eb945c574ff0adf459ebd49e9705dd1ed33d))
- Update element handling to use views instead of diagrams and adjust related tests ([`5c76e51`](https://github.com/Mertcikla/tld-cli/commit/5c76e51e7cf6c0e33915a40a26756dc0ae7f2a3a))
- Update YAML marshalling to use pretty formatting and adjust build output path ([`2ca1e9f`](https://github.com/Mertcikla/tld-cli/commit/2ca1e9f03be070e8b926ac34863e17fcf97ccce2))
- Add JSON output mode and --changed-since incremental analyze ([`8e0627c`](https://github.com/Mertcikla/tld-cli/commit/8e0627cce953540f876c09df80f12c3814d2c2d3))
- Implement repo-scoped analysis and configuration ([`2659c16`](https://github.com/Mertcikla/tld-cli/commit/2659c16546fba112f55ee865e65b798ec08f83b8))
- Add WASM-based symbol extraction for multiple languages ([`e99e59a`](https://github.com/Mertcikla/tld-cli/commit/e99e59a43b9da4d2db9bf25904a70ce3226a4b14))
- Infer connector view from element placements and default to root ([`f0617af`](https://github.com/Mertcikla/tld-cli/commit/f0617af422bbc6676a31c6843db09b242dbc8b6e))
- Introduce elements and connectors to workspace management ([`08b8caa`](https://github.com/Mertcikla/tld-cli/commit/08b8caafc03bf5fa246ae63976eeea446b67cb84))

### Refactoring

- Use Rust let-chains and add OnEntry callback type ([`3f25bab`](https://github.com/Mertcikla/tld-cli/commit/3f25bab4cccc50da0152ea1798af6a4e83ec7e45))
- Migrate project from Go to Rust ([`67f40c4`](https://github.com/Mertcikla/tld-cli/commit/67f40c491586c56e6348f0720d45596fba015a17))
- Update analyzer logic and refresh test codebase examples ([`96c9a9f`](https://github.com/Mertcikla/tld-cli/commit/96c9a9fc870ad8af2570c3bbe317faa4e4c0e8c4))
- Update analyzer logic and planner reporting while initializing git repositories for test-codebase examples ([`6627bed`](https://github.com/Mertcikla/tld-cli/commit/6627bed55d46f756a46215fa57507829023d1247))
- Update CLI commands and enhance test codebase examples with configuration and git metadata ([`2ebfb56`](https://github.com/Mertcikla/tld-cli/commit/2ebfb56e36e48b18379d3782211e41bd9dc68ad8))
- Rename create element to add and simplify connect command syntax ([`dd3691d`](https://github.com/Mertcikla/tld-cli/commit/dd3691d5b017b7680e7dd2a454232f8874ccce2b))
- Remove deprecated commands and update workspace structure ([`958713a`](https://github.com/Mertcikla/tld-cli/commit/958713aaf2132e8948a70938456beccc27cb1d35))

## [0.1.7] - 2026-04-04

### Bug Fixes

- Lint warnings & error handling ([`27cabae`](https://github.com/Mertcikla/tld-cli/commit/27cabae5b39dce37fdaea3b56fd30d990c4203c8))

## [0.1.6-beta.1] - 2026-03-31

### Documentation

- Rename add link to create link and document update commands ([`c827779`](https://github.com/Mertcikla/tld-cli/commit/c827779ab7837379ea36f3f7ef1f1b388d53f094))
- Polish create-diagram guide, fix punctuation and clarify examples ([`790f99c`](https://github.com/Mertcikla/tld-cli/commit/790f99c6d2be6266fdadfc342485b6a4711a537a))

### Features

- Add version command, Homebrew tap support, and tech icon definitions ([`9c459a2`](https://github.com/Mertcikla/tld-cli/commit/9c459a26d2135f97d6273c9cc0feb918023b6945))
- Add source linking support and 'update source' command ([`016c8b5`](https://github.com/Mertcikla/tld-cli/commit/016c8b5c750aabf9dab73622ab725f6c826ebd26))
- Add tldiagram plugins, code-explorer agent, and diagram skill ([`b9be6b6`](https://github.com/Mertcikla/tld-cli/commit/b9be6b66a61d550c9bf8e685f11feda7a8544bf8))

## [0.1.5] - 2026-03-28

### Bug Fixes

- Run semantic-release with plugins and push tags using GH_TOKEN ([`d532ed4`](https://github.com/Mertcikla/tld-cli/commit/d532ed47b4210729f0c1b94ffbea178f2aed1214))
- Ci use GH_TOKEN secret for semantic-release in tag workflow ([`11009cf`](https://github.com/Mertcikla/tld-cli/commit/11009cfbc1e1568b3f7bc94c76520ec3900cd808))
- Remove @semantic-release/github plugin from release config ([`af15229`](https://github.com/Mertcikla/tld-cli/commit/af15229f8aee05acf0766b0adb7abab411756b9d))

### Features

- Add architectural warnings and --strictness flag to plan/validate ([`dba9e35`](https://github.com/Mertcikla/tld-cli/commit/dba9e35e2c4b4eff0bbafa4b0a48c708fd22a7ea))
- Add dry-run conflict tests, plan -v flag, and validation config ([`b2c4df9`](https://github.com/Mertcikla/tld-cli/commit/b2c4df94c5e5dc4344c7afb9b70d18fefbaeb1b5))

## [0.1.2] - 2026-03-28

### Documentation

- Update README.md ([`9fc3f76`](https://github.com/Mertcikla/tld-cli/commit/9fc3f76fb8a1f513a1604530b6679ea481e4199f))

### Features

- Implement idempotent edge upserts via stable metadata references and update create-diagram skill documentation ([`4e03c20`](https://github.com/Mertcikla/tld-cli/commit/4e03c20f867dc4643af2e542de97341916d0c9fb))

## [0.1.0] - 2026-03-27


