# Anesis

Anesis is a Rust CLI for scaffolding JavaScript and TypeScript projects from remote Anesis templates and extending them with project addons.

It supports:

- creating a new project from a template
- checking cached templates and addons for updates before use
- authenticating against the Anesis service
- publishing GitHub repositories as Anesis templates
- installing cached addons and running addon commands inside a project
- upgrading the CLI from GitHub Releases with `anesis upgrade`
- notifying you when a newer Anesis version is available

## Official website

The official Anesis website is [anesis.vercel.app](https://anesis.vercel.app/).

Use it as the main guide for learning and using Anesis:

- [Docs](https://anesis.vercel.app/docs) cover installation, authentication, templates, addons, publishing, and reference material.
- [Templates](https://anesis.vercel.app/templates) lists available project starters.
- [Addons](https://anesis.vercel.app/addons) lists reusable workflow addons.

This README keeps a quick CLI reference for the repository and package users. The website contains the full, up-to-date usage guide.

## Installation

### Quick install

Linux and macOS:

```bash
curl -sSL https://raw.githubusercontent.com/anesis-dev/anesis/main/install.sh | bash
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/anesis-dev/anesis/main/install.ps1 | iex
```

Unix installers place the `anesis` binary in `~/.local/bin`. Cargo installs it in `~/.cargo/bin`. Make sure the relevant directory is in your `PATH`.

### Install with npm

```bash
npm install -g anesis-cli
```

The npm package downloads the matching prebuilt Anesis binary during `postinstall`.

### Install with cargo

```bash
cargo install anesis
```

### Manual install from GitHub Releases

Download the latest release artifact for your platform:

- [Linux x86_64](https://github.com/anesis-dev/anesis/releases/latest/download/anesis-linux-x86_64.tar.gz)
- [Linux ARM64](https://github.com/anesis-dev/anesis/releases/latest/download/anesis-linux-aarch64.tar.gz)
- [macOS Apple Silicon](https://github.com/anesis-dev/anesis/releases/latest/download/anesis-macos-aarch64.tar.gz)
- [Windows x86_64](https://github.com/anesis-dev/anesis/releases/latest/download/anesis-windows-x86_64.zip)

## Getting started

Most remote operations require authentication first:

```bash
anesis login
```

Create a new project from a template:

```bash
anesis new my-app react-vite-ts
```

Anesis checks whether the cached template is current before generating the project. If a newer template version exists, it updates the local cache first.

## Command overview

Top-level commands:

```text
anesis new <NAME> <TEMPLATE_NAME>
anesis template <COMMAND>
anesis login
anesis logout
anesis account
anesis addon <COMMAND>
anesis upgrade
anesis completions <SHELL>
anesis use <ADDON_ID> <COMMAND>
```

Template management:

```text
anesis template install <TEMPLATE_NAME>
anesis template list
anesis template remove <TEMPLATE_NAME>
anesis template publish <GITHUB_REPOSITORY_URL>
anesis template update <GITHUB_REPOSITORY_URL>
```

Addon management:

```text
anesis addon install <ADDON_ID>
anesis addon list
anesis addon remove <ADDON_ID>
anesis addon publish <GITHUB_REPOSITORY_URL>
anesis addon update <GITHUB_REPOSITORY_URL>
```

Addon execution:

```text
anesis use <ADDON_ID> <COMMAND>
```

Example:

```bash
anesis addon install drizzle
cd my-app
anesis use drizzle init
```

Upgrade Anesis itself:

```bash
anesis upgrade
```

Install shell completions:

```bash
anesis completions zsh
```

Supported shells are `bash`, `zsh`, `fish`, and `powershell`.

Aliases:

- `anesis n ...` for `anesis new ...`
- `anesis t ...` for `anesis template ...`
- `anesis a ...` for `anesis addon ...`
- `anesis in` for `anesis login`
- `anesis out` for `anesis logout`

## Common workflows

Install or refresh a template in the local cache:

```bash
anesis template install react-vite-ts
```

When you run `anesis new`, Anesis also refreshes the template cache automatically if a newer version is available.

List cached templates:

```bash
anesis template list
```

Remove a cached template:

```bash
anesis template remove react-vite-ts
```

Show the authenticated account:

```bash
anesis account
```

Publish a GitHub repository as a template:

```bash
anesis template publish https://github.com/owner/repo
```

Update a published template:

```bash
anesis template update https://github.com/owner/repo
```

Install an addon:

```bash
anesis addon install drizzle
```

Run an installed addon command:

```bash
anesis use drizzle init
```

When you run an addon command such as `anesis use drizzle init`, Anesis checks for a newer cached addon version first. If the add-on updated and the command is marked `once: true`, Anesis prompts you to re-run it.

List installed addons:

```bash
anesis addon list
```

Remove a cached addon:

```bash
anesis addon remove drizzle
```

Publish a GitHub repository as an addon:

```bash
anesis addon publish https://github.com/owner/repo
```

Update a published addon:

```bash
anesis addon update https://github.com/owner/repo
```

Upgrade the CLI to the latest release:

```bash
anesis upgrade
```

After most commands, Anesis performs a background version check and prints a short upgrade notice when a newer CLI release is available.

Install shell completions:

```bash
anesis completions zsh
```

Supported shells are `bash`, `zsh`, `fish`, and `powershell`.

## Local data and generated files

Anesis stores local state under `~/.anesis/`:

- cached templates in `~/.anesis/cache/templates`
- cached addons in `~/.anesis/cache/addons`
- template cache index in `~/.anesis/cache/templates/anesis-templates.json`
- addon cache index in `~/.anesis/cache/addons/anesis-addons.json`
- CLI version-check cache in `~/.anesis/version_check.json`
- authentication data in `~/.anesis/auth.json`

When addon commands run inside a project, Anesis records execution state in `anesis.lock` in the project root, including the add-on version used for each executed command.

## Templates

Published templates are expected to include an `anesis.template.json` manifest in the template root. Anesis uses that manifest to track template metadata such as template name, version, and source repository.

Template files ending with `.tera` are rendered during project generation and written without the `.tera` suffix.

Available template variables:

- `project_name`
- `project_name_kebab`
- `project_name_snake`

## Addons

Installed addons are expected to include an `anesis.addon.json` manifest. Anesis uses addon manifests to define:

- user inputs
- project detection rules
- command variants
- file modification steps such as create, copy, inject, replace, append, delete, rename, and move

## License

Licensed under either of these:

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License 2.0 ([LICENSE-Apache](LICENSE-Apache))
