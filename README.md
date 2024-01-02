# Uploadstr

![](https://img.shields.io/badge/rust-stable-blue.svg?logo=rust)
![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/terryyrret/uploadstr/CI-CD.yml)
[![GitHub Issues](https://img.shields.io/github/issues/terryyrret/uploadstr.svg)](https://github.com/terryyrret/uploadstr/issues)
![Contributions welcome](https://img.shields.io/badge/contributions-welcome-orange.svg)
[![License](https://img.shields.io/badge/license-GNU_AGPL_v3-blue.svg)](https://opensource.org/license/agpl-v3/)

Static file server that allows for uploading files, deleting files, viewing a list of uploaded files, or statically serving files from a URL. Uploading, deleting, and viewing a list of files uses Nostr HTTP Auth (NIP-98) to authenticate the user.


## Features
- RESTful API Endpoints
  - /list - get a list of files stored on the server (requires Nostr auth)
  - /delete - delete a file stored on the server (requires Nostr auth)
  - /upload - upload a file to the server (requires Nostr auth)
  - /f - statically serve files stored on the server (freely served to anyone without any auth)
  - Check wiki for documentation on these endpoints.
- Docker container to easily spin up and run service
- Configurable pubkey whitelist to configure who can use the /list, /delete, /upload endpoints.

## Config file
Uploadstr looks for a JSON config file stored at /etc/uploadstr/config. It does not generate one by default so you'll have to create one and put it there. Here is a sample config file.

config
``` json
{
    "bind": "0.0.0.0:3000",
    "baseUrl": "https://img.yrret.me",
    "filesDir": "/var/uploadstr/files",
    "pubkeyWhitelist": [
        "5e46320e8cb01f15a15674106289f9c9bf2ac0470f06bef0efb165959ea084de"
    ]
}
```

Here, we can see this JSON file has only 4 keys. Below is a description of each key.

| Key | Description |
|-----|---------------|
| bind | Address for the service to listen for requests on |
| baseUrl | The base URL that the service is publically exposed on. This string is used to verify Nostr HTTP Auths | 
| filesDir | The directory for the service to look for and store its files |
| pubkeyWhitelist | An array of pubkeys encoded as hex strings that serves as a whitelist for who can access the auth protected endpoints |

## How to setup the docker container
This repository has a docker workflow that automatically builds docker images containing the uploadstr binary. It is provided for convenience for those that want to use container images. You could instead use the provided binaries or compile them yourself. But, here is a sample docker compose file:

docker-compose.yml
``` yaml
version: "3"
services:
  uploadstr:
    container_name: uploadstr
    image: ghcr.io/terryyrret/uploadstr:latest
    volumes:
      - /Some/Dir/to/files:/var/uploadstr/files
      - /Some/Dir/to/config:/etc/uploadstr/config
```
