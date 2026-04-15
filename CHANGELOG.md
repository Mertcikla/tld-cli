# Changelog

All notable changes to tld are documented here.

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


