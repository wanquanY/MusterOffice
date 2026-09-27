# MusterOffice development plugin

This package combines the presentation Skill with the same native computation
MCP and matching export worker. It is a local development build, not a published
or signed release. Full presentation/Office/WPS and native platform acceptance
remain in progress. No account, database, upload service or product UI is added.

The portable `plugin.json`/`mcp.json` target Agent Plugins 1.0.0. The generated
Codex compatibility files project the same identity, Skill and server. Clients
must support local stdio binaries for the package's OS/architecture. A manifest
check does not establish that a particular client installed or activated it.

The host creates its protected input, output and temporary directories, then
writes `caller-files.json` in its selected `PLUGIN_DATA` directory:

```json
{
  "inputDirectory": "/absolute/caller/inputs",
  "outputDirectory": "/absolute/caller/outputs",
  "temporaryDirectory": "/absolute/caller/temporary",
  "computationSlots": 2,
  "controlSlots": 2
}
```

These are caller-owned paths, not a MusterOffice content library. Input/output
file tools or an attachment bridge must give the Agent access to the chosen
channel. The temporary tree must be separate. Do not put `exportWorker` in this
configuration: package startup selects the matching bundled worker and verifies
its recorded hash. Keep directories protected/stable during calls.

Install or register this directory using the chosen client's own workflow. No
installation hook, credential, network fetch, global config edit or background
service is included. The process reads configuration and uses pipes supplied by
the client. Stdio supports MCP 2025-11-25 and 2026-07-28.

Start with capability/schema discovery, then use the bundled Skill. Tool calls
complete or fail within their lifetime; outputs report `productCommitted: false`.
The host must save/register output according to its rules. Upgrade can replace
the package without rewriting caller paths; existing processes should drain
before the host changes their executable/configuration. Uninstall removes only
the client registration/package, never caller documents. Do not store user PPTX
files in a plugin-managed directory that the client may delete on uninstall.

`bundle-manifest.json` inventories bytes and executable flags. Its hashes detect
assembly damage, not publisher authenticity. `notices/` preserves recorded
upstream texts; complete transitive licensing, signing and distribution clearance
are still pending. Font/media bytes are supplied explicitly by the caller and
are not installed from this Skill. Development artifact sizes are not production
package or Musterwork bundle-size measurements.
