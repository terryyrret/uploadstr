# Uploadstr

![](https://img.shields.io/badge/rust-stable-blue.svg?logo=rust)
![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/terryyrret/uploadstr/CI-CD.yml)
[![GitHub Issues](https://img.shields.io/github/issues/terryyrret/uploadstr.svg)](https://github.com/terryyrret/uploadstr/issues)
![Contributions welcome](https://img.shields.io/badge/contributions-welcome-orange.svg)
[![License](https://img.shields.io/badge/license-GNU_AGPL_v3-blue.svg)](https://opensource.org/license/agpl-v3/)

Static file server that allows for uploading files, deleting files, viewing a list of uploaded files, or statically serving files from a URL. Uploading, deleting, and viewing a list of files uses Nostr HTTP Auth (NIP-98) to authenticate the user.


### Features
- RESTful API Endpoints
  - /list - get a list of files stored on the server (requires Nostr auth)
  - /delete - delete a file stored on the server (requires Nostr auth)
  - /upload - upload a file to the server (requires Nostr auth)
  - /f - statically serve files stored on the server (freely served to anyone without any auth)
  - Check wiki for documentation on these endpoints.
- Docker container to easily spin up and run service
- Configurable pubkey whitelist to configure who can use the /list, /delete, /upload endpoints.
