# BPL for JetBrains IDEs

Syntax highlighting and glyph-entry templates for IntelliJ IDEA, PyCharm, WebStorm and CLion.

## Syntax highlighting

1. Enable **TextMate Bundles** under **Settings > Plugins**.
2. Under **Settings > Editor > TextMate Bundles**, click **+** and select this folder's `BPL.tmbundle` directory.
3. Open a `.bpl` file.

The bundle contains the same generated TextMate grammar used by BPL's playground. See JetBrains' [TextMate instructions](https://www.jetbrains.com/help/idea/textmate.html).

## Glyph entry

Quit the IDE, then copy `BPL.xml` into the `templates` subdirectory of its configuration directory. Create `templates` if it does not exist.

| OS | Default configuration directory |
|---|---|
| macOS | `~/Library/Application Support/JetBrains/<product><version>/` |
| Linux | `~/.config/JetBrains/<product><version>/` |
| Windows | `%APPDATA%\JetBrains\<product><version>\` |

For example, `<product><version>` can be `IntelliJIdea2026.2` or `PyCharm2026.2`. Find your actual path under **Help > Diagnostic Tools > Special Files and Folders**. JetBrains documents [configuration directories](https://www.jetbrains.com/help/idea/directories-used-by-the-ide-to-store-settings-caches-plugins-and-logs.html) and [sharing live templates](https://www.jetbrains.com/help/idea/sharing-live-templates.html).

Restart the IDE. The **BPL** group appears under **Settings > Editor > Live Templates**. Type a glyph name or alias, then press Tab: `iota` becomes `⍳`, `reshape` becomes `⍴`, and `range` becomes `↦`. Hyphenated names such as `grade-up` work too.

These templates are available in every file, not only `.bpl` files. They use ordinary names without a prefix. Change or disable individual templates in Live Templates settings to resolve conflicts with your other templates.

## Plugin reuse

Copy `BPL.xml` into your plugin's `src/main/resources/liveTemplates/` directory and register it in `plugin.xml`:

```xml
<extensions defaultExtensionNs="com.intellij">
  <defaultLiveTemplates file="/liveTemplates/BPL.xml"/>
</extensions>
```

The XML uses JetBrains' `OTHER` (Everywhere) context. To limit applicability, replace those entries with your plugin's registered context. See [Providing Live Templates](https://plugins.jetbrains.com/docs/intellij/providing-live-templates.html).

## Generation

`basedpl.editors.write()`, called by `scripts/prep.py`, generates the templates from BPL's shared glyph names and aliases and copies the shared grammar into the bundle. Edit the generators or source metadata, not the generated files.
