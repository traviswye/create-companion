# Catalog expansion report

Generated 2026-09-04 on branch `catalog-expansion` by `tools/gen_presets.py` from `catalog/*.json` (134 source files) plus the ShortcutMapper import.

**160 entries** (109 apps, 47 sites, 4 systems), **22,622 actions**. Validator: 0 errors. QA: 60/60 sampled shortcuts matched their sources across four rounds. QA: 60/60 sampled shortcuts matched their sources across four rounds. Rust gate (`apps_catalog_chords_parse`): every chord parses. Column check: 0 Mac chords in Windows columns and vice versa.

## Quality checks

| Wave | Files sampled | Result |
|---|---|---|
| 1 | google_docs, vscode, chrome | 15/15 sampled shortcuts match the source; counts consistent |
| 2 | excel, final_cut_pro, teams | 15/15 match; two Final Cut sections match row-for-row |
| 3 | ableton_live, godot, canva | 15/15 match; one vendor typo silently normalised in canva (Option+P on the Windows tab) |
| 4 | acrobat, paint_net, outlook_web | 15/15 match; Acrobat diffed section-by-section with no gaps; one undocumented omission in outlook_web (Invite Attendees = N, same key as an existing action) |

## Entries

Count = actions in the generated catalog. Twins (`<id>_web`) are browser versions generated from the desktop entry.

### AI

| id | name | kind | actions | sources | note |
|---|---|---|---|---|---|
| `chatgpt` | ChatGPT | site | 50 | 1 | OpenAI's Help Center article this catalog entry was originally scoped to ('Keyboard shortcuts in ChatGPT', the list shown in-app by Ctrl+... |

### Browser

| id | name | kind | actions | sources | note |
|---|---|---|---|---|---|
| `brave` | Brave | app | 74 | 1 | From Brave's Help Center article "What keyboard shortcuts can I use in Brave?", which is itself Brave's own restatement of the Chromium s... |
| `browser` | Browser | app | 7 | 1 | Combined browser profile used by the first-run config (Chromium-family chords). Per-browser entries chrome/edge/firefox/safari/brave/viva... |
| `chrome` | Google Chrome | app | 87 | 1 | From Google's "Chrome keyboard shortcuts" support page (Windows/Linux and Mac tabs). Where the vendor lists multiple alternate chords for... |
| `edge` | Microsoft Edge | app | 96 | 1 | From Microsoft's "Keyboard shortcuts in Microsoft Edge" support page. Where the page lists more than one chord for an action, only the pr... |
| `firefox` | Mozilla Firefox | app | 73 | 3 | The current SUMO article (first source URL, named in the brief) returned Mozilla's bot-verification "Client Challenge" interstitial to ev... |
| `safari` | Safari | app | 48 | 1 | From Apple's Safari User Guide, "Keyboard shortcuts and gestures" page (Mac only). Skipped as not expressible: every gesture and mouse/tr... |
| `vivaldi` | Vivaldi | app | 46 | 4 | Vivaldi's own "Keyboard Shortcuts" help page does not publish a text list of the default chord shortcuts: it only points to an in-app che... |

### CAD / 3D

| id | name | kind | actions | sources | note |
|---|---|---|---|---|---|
| `substance_painter` | Adobe Substance 3D Painter | app | 67 | 2 | From the Substance 3D Painter documentation "Shortcuts" page, which is split into "Editable shortcuts" and "Non-editable shortcuts"; both... |
| `3dsmax` | Autodesk 3ds Max | app | 433 | 1 |  |
| `autocad` | Autodesk AutoCAD | app | 49 | 2 | From the Autodesk "AutoCAD Keyboard Shortcuts Guide" (autodesk.com/shortcuts/autocad and the PDF it links to), sections "Toggle General F... |
| `fusion360` | Autodesk Fusion 360 | app | 156 | 2 | From the Autodesk Knowledge Network topic "Fusion keyboard shortcuts reference". Mouse-driven bindings are omitted because the chord gram... |
| `maya` | Autodesk Maya | app | 131 | 1 |  |
| `blender` | Blender | app | 745 | 5 | Default bindings hand-authored 2026-09-04; the full shortcut list comes from the ShortcutMapper import in reference/app-shortcuts.json. E... |
| `cinema4d` | Maxon Cinema 4D | app | 250 | 2 | From the Cinema 4D "Keyboard Shortcuts" page of the Maxon help. That page states "Mac users please press the CMD key instead of the CTRL ... |
| `zbrush` | Maxon ZBrush | app | 96 | 3 | From the ZBrush user guide "Shortcuts by Category" page (the docs.pixologic.com pages now redirect to help.maxon.net/zbr). The page lists... |
| `houdini` | SideFX Houdini | app | 584 | 1 |  |
| `sketchup` | SketchUp | app | 39 | 1 |  |
| `solidworks` | SOLIDWORKS | app | 34 | 3 | SOLIDWORKS is Windows only, so there is no `mac` entry anywhere. The entries with a context come from the SOLIDWORKS 2025 Help topic "Sel... |

### Communication

| id | name | kind | actions | sources | note |
|---|---|---|---|---|---|
| `discord` | Discord | app | 43 | 2 | Skipped: Toggle mentions popout (Ctrl/Cmd+Alt+@) — @ is not an expressible key in this grammar. 'Start/stop inline code block' has no doc... |
| `discord_web` | Discord (web) | site | 43 | 2 | twin of `discord` |
| `gmail` | Gmail | site | 87 | 1 | Most shortcuts under Actions, Jumping, Threadlist Selection, Navigation and Application require Settings > General > Keyboard shortcuts: ... |
| `google_meet` | Google Meet | site | 16 | 2 | Google does not publicly document a keyboard shortcut to leave a call. |
| `teams` | Microsoft Teams | app | 167 | 1 | From the official 'Keyboard shortcuts for Microsoft Teams' Microsoft Support article (Windows and MacOS tabs), current as of 2026-09. Onl... |
| `thunderbird` | Mozilla Thunderbird | app | 179 | 1 | From the official Mozilla Support 'Keyboard shortcuts' article for Thunderbird (last updated 2026-04-02). The source states the shortcut ... |
| `outlook_web` | Outlook on the web | site | 94 | 1 | "Outlook on the web" tab only (the New Outlook and Classic Outlook desktop tabs on this same page are already covered by catalog/outlook.... |
| `signal` | Signal Desktop | app | 66 | 2 | From the official 'Signal Desktop Keyboard Shortcuts' Help Center article (Mac and Windows/Linux columns), current as of 2026-09; the 'Te... |
| `slack` | Slack | app | 99 | 1 | From the official 'Slack keyboard shortcuts' Help Center article, current as of 2026-09. Skipped as not expressible or not a fixed shortc... |
| `slack_web` | Slack (web) | site | 99 | 1 | twin of `slack` |
| `telegram` | Telegram Desktop | app | 51 | 4 | No single canonical shortcuts page exists on telegram.org or desktop.telegram.org (the FAQ does not list desktop shortcuts, and Telegram'... |
| `whatsapp` | WhatsApp Desktop | app | 37 | 2 | From the official WhatsApp Help Center 'About keyboard shortcuts' article, current as of 2026-09, using its Windows and Mac tabs (the art... |
| `zoom` | Zoom Workplace | app | 86 | 1 | From the official 'Using hot keys and keyboard shortcuts' support article (support.zoom.com), current as of 2026-09. Windows and macOS se... |

### Creative

| id | name | kind | actions | sources | note |
|---|---|---|---|---|---|
| `animate` | Adobe Animate | app | 52 | 2 | Every row of Adobe's "Adobe Animate keyboard shortcuts" page, whose five tables (File, View, Windows, Edit and modify, Miscellaneous acti... |
| `illustrator` | Adobe Illustrator | app | 217 | 1 |  |
| `indesign` | Adobe InDesign | app | 229 | 2 | Every row of Adobe's InDesign keyboard-shortcuts page (helpx.adobe.com/indesign/using/default-keyboard-shortcuts.html now redirects to th... |
| `lightroom` | Adobe Lightroom | app | 607 | 2 | This profile covers Adobe Lightroom Classic as well as Lightroom: the match rules list both Lightroom.exe and LightroomClassic.exe, so th... |
| `photoshop` | Adobe Photoshop | app | 708 | 4 | Default bindings hand-authored 2026-09-04; the full shortcut list comes from the ShortcutMapper import in reference/app-shortcuts.json. E... |
| `affinity_designer` | Affinity Designer 2 | app | 219 | 2 | Every row of Serif's "Keyboard shortcuts" page in the Affinity Designer 2 help. The context is the page's category with the trailing word... |
| `affinity_photo` | Affinity Photo 2 | app | 248 | 2 | Every row of Serif's "Keyboard shortcuts" page in the Affinity Photo 2 help. The context is the page's category with the trailing word "s... |
| `affinity_publisher` | Affinity Publisher 2 | app | 183 | 2 | Every row of Serif's "Keyboard shortcuts" page in the Affinity Publisher 2 help. The context is the page's category with the trailing wor... |
| `canva` | Canva | site | 145 | 2 | The source page's 'Open More actions menu for a selected element' row renders as 'Shift + 10' on both platform tabs (the leading F of F10... |
| `capture_one` | Capture One Pro | app | 94 | 60 | Capture One does not publish its default shortcut set as a document. The vendor's Help Center has a "Keyboard Shortcuts" section (section... |
| `clip_studio_paint` | CLIP STUDIO PAINT | app | 94 | 7 | Every row of the "Shortcut list" chapter of the official CLIP STUDIO PAINT user guide (Ver. 4.0 online manual, read 2026-09-04): the Menu... |
| `darktable` | darktable | app | 138 | 47 | Read from the darktable 5.6 user manual (the current released manual). darktable has no single "default shortcuts" table: the shortcuts p... |
| `figma` | Figma | app | 107 | 25 | Figma no longer publishes one master 'all shortcuts' Help Center page (it moved the full list into an in-app panel opened with Ctrl+Shift... |
| `figma_web` | Figma (web) | site | 107 | 25 | twin of `figma` |
| `gimp` | GIMP | app | 131 | 16 | Taken from the GIMP 3.2 manual's "Keys and Mouse Reference" appendix (the per-menu key-reference pages for File, Edit, Select, View, Imag... |
| `inkscape` | Inkscape | app | 294 | 1 | From Inkscape's official "Inkscape keyboard and mouse reference" (version 1.4.x, last revised 2024-10-05), which mirrors share/inkscape/k... |
| `krita` | Krita | app | 127 | 33 | Krita's manual has no single default-shortcut table, so this list is assembled from the pages of the manual that state a default: the Mai... |
| `miro` | Miro | app | 77 | 2 | Covers both the Miro website and the Miro desktop app (same keymap per the vendor's shortcuts article). Not expressible, so left out: dup... |
| `miro_web` | Miro (web) | site | 77 | 2 | twin of `miro` |
| `paint_net` | paint.net | app | 210 | 1 | Every keyboard command on the "Keyboard & Mouse Commands" page of the Paint.NET documentation (revision date 3 August 2024). paint.net is... |

### Development

| id | name | kind | actions | sources | note |
|---|---|---|---|---|---|
| `android_studio` | Android Studio | app | 139 | 1 | From the Android Developers "Keyboard shortcuts" reference for Android Studio (default keymap), Windows/Linux and macOS columns; Linux eq... |
| `docker_desktop` | Docker Desktop | app | 5 | 1 | Docker Desktop has no dedicated keyboard-shortcut reference page. These are the only shortcuts Docker documents, drawn from its official ... |
| `github` | GitHub | site | 126 | 2 | Extracted from GitHub Docs 'Keyboard shortcuts' (17 sections) and 'GitHub Command Palette' pages, 2026-09-04. Where a shortcut was docume... |
| `gitkraken` | GitKraken Desktop | app | 52 | 1 | From the official GitKraken Desktop keyboard-shortcuts reference. Alternate bindings printed beside a primary one are omitted in favour o... |
| `gitlab` | GitLab | site | 138 | 2 | Two-key shortcuts written as "g"+"x" style throughout the doc (Project navigation, and the "c"+"r" copy-reference shortcuts repeated in I... |
| `emacs` | GNU Emacs | app | 197 | 3 | From the GNU Emacs Reference Card - parsed from etc/refcards/refcard.tex in the Emacs tree, the typeset source of the PDF the GNU project... |
| `godot` | Godot Engine | app | 211 | 2 | From the official Godot Engine 4.5 docs ("stable" branch resolves to 4.5 as of this writing) 'Default editor shortcuts' page, which the d... |
| `iterm2` | iTerm2 | app | 75 | 2 | From the official iTerm2 documentation (one-page manual, current as of iTerm2 3.6.x docs, fetched 2026-09-04). iTerm2's docs do not publi... |
| `appcode` | JetBrains AppCode | app | 225 | 1 |  |
| `clion` | JetBrains CLion | app | 234 | 1 |  |
| `intellij` | JetBrains IntelliJ IDEA | app | 250 | 1 |  |
| `phpstorm` | JetBrains PhpStorm | app | 238 | 1 |  |
| `pycharm` | JetBrains PyCharm | app | 238 | 1 |  |
| `rubymine` | JetBrains RubyMine | app | 245 | 1 |  |
| `webstorm` | JetBrains WebStorm | app | 236 | 1 |  |
| `neovim` | Neovim | app | 65 | 16 | This file is a DELTA against catalog/vim.json, which already fully covers Vim's own Normal/Insert/Visual/Command-line commands (Neovim in... |
| `notepad_plus_plus` | Notepad++ | app | 166 | 5 | Windows only. The Notepad++ User Manual describes the Shortcut Mapper but does not print the Main menu tab's default assignments, so this... |
| `postman` | Postman | app | 66 | 2 | Postman's Learning Center retired its dedicated 'Keyboard Shortcuts' reference page (learning.getpostman.com/docs/postman/launching_postm... |
| `sublime` | Sublime Text | app | 183 | 5 | Sublime Text 4. Taken from the key bindings Sublime ships inside Packages/Default.sublime-package - Default (Windows).sublime-keymap and ... |
| `terminal` | Terminal | app | 63 | 1 | Windows Terminal only; the docs give no macOS bindings, so all actions are windows-only. Includes only commands that have an explicit def... |
| `tmux` | tmux | app | 182 | 3 | tmux itself has no exe/bundle - it's a program running inside a terminal emulator, so this profile only fires when a Windows terminal's w... |
| `unity` | Unity | app | 39 | 1 |  |
| `unreal_engine` | Unreal Engine | app | 57 | 7 | From Epic's official Unreal Engine 5.8 documentation (Viewport Controls, Viewport Modes, Level Editor, Selecting Actors, Transforming Act... |
| `vim` | Vim | app | 652 | 3 | From Vim's own :help index (runtime/doc/index.txt, the source behind vimhelp.org/index.txt.html): section 1 Insert mode, section 2 Normal... |
| `visual_studio` | Visual Studio | app | 766 | 2 | Visual Studio 2022, General Development profile, from the Microsoft Learn "Keyboard shortcuts in Visual Studio" reference (Popular, Globa... |
| `vscode` | VS Code | app | 150 | 3 | From the official VS Code keyboard-shortcut reference cards for Windows and macOS. Mouse-only bindings are omitted: Insert cursor (Alt+Cl... |
| `xcode` | Xcode | app | 233 | 4 | Mac only. From Apple's "Xcode Keyboard Shortcuts and Gestures" reference: the "Menu Command Shortcuts (By Menu)" tables (Xcode, File, Edi... |

### Files

| id | name | kind | actions | sources | note |
|---|---|---|---|---|---|
| `dropbox_web` | Dropbox | site | 5 | 1 | Web only (dropbox.com); the source states these shortcuts work only while viewing the Files tab of dropbox.com, and are the same on Windo... |
| `windows_explorer` | File Explorer | app | 20 | 1 | The File Explorer section of Microsoft's shortcuts page, plus F3 and F4, which that page's general section documents as File Explorer com... |
| `finder` | Finder | app | 53 | 1 | The Finder rows of Apple's "Finder and system shortcuts" section, plus the Finder-specific rows in the common-shortcuts section. Apple wr... |
| `google_drive` | Google Drive | site | 66 | 1 | Google Drive on the web, current UI. Two-key navigation shortcuts (e.g. 'g then n') are modeled as sequences; where the vendor's Mac colu... |

### Games

| id | name | kind | actions | sources | note |
|---|---|---|---|---|---|
| `ets2` | Euro Truck Simulator 2 | app | 47 | 1 |  |
| `steam` | Steam | app | 3 | 2 | Steam Support does not publish a dedicated 'Steam client keyboard shortcuts' article (the FAQ id given in the task, 1B3B-6D0D-7C4D-8AC9, ... |

### Music

| id | name | kind | actions | sources | note |
|---|---|---|---|---|---|
| `ableton_live` | Ableton Live | app | 304 | 1 | Every row of chapter 41, "Live Keyboard Shortcuts", of the Ableton Live 12 reference manual. The context is Ableton's own section name wi... |
| `audition` | Adobe Audition | app | 30 | 2 | Every row of Adobe's "Default keyboard shortcuts" page for Audition (helpx.adobe.com/audition/using/default-keyboard-shortcuts.html now r... |
| `apple_music` | Apple Music | app | 89 | 2 | Windows access keys are documented as sequential Alt-mnemonic presses (Alt, then two letters, one after another). They are encoded here a... |
| `audacity` | Audacity | app | 96 | 1 | Only the default 'Standard' keyboard shortcut set is included, per the manual; the extended 'Full' set (the shortcuts available in Audaci... |
| `pro_tools` | Avid Pro Tools | app | 622 | 1 | Every keyboard row of Avid's "Pro Tools Shortcuts Guide", version 2025.6 (created 5/20/2025). The guide prints an Action column and separ... |
| `fl_studio` | FL Studio | app | 258 | 1 | Every keyboard row of Image-Line's "Keyboard & Mouse Shortcuts" page of the FL Studio online manual. The context is the manual's own tabl... |
| `foobar2000` | foobar2000 | app | 9 | 1 | Windows only; there is no official macOS build. foobar2000's keyboard shortcuts are entirely user-assignable via Preferences > Keyboard S... |
| `logic_pro` | Logic Pro | app | 768 | 3 | Every row of the "Key command tables" section of Apple's Logic Pro User Guide for Mac (the "Default keyboard shortcuts" pages, read from ... |
| `musicbee` | MusicBee | app | 50 | 1 | Windows only; there is no official macOS build. MusicBee offers 150+ assignable hotkey commands, but only the ones listed here are bound ... |
| `reaper` | REAPER | app | 279 | 3 | REAPER has no single shortcut appendix in the current user guide: the guide points at Help > Keybindings and Mouse Modifiers (Shift+F1), ... |
| `soundcloud` | SoundCloud | site | 31 | 1 | SoundCloud has no separate written help article for these; the list was read from the in-app overlay (press H on soundcloud.com). The ove... |
| `spotify` | Spotify | app | 37 | 1 | The support article documents mouse-free in-app shortcuts only; it does not currently list media-key style shortcuts for seek or previous... |
| `spotify_web` | Spotify (web) | site | 37 | 1 | twin of `spotify` |
| `cubase` | Steinberg Cubase | app | 212 | 4 | Every row of the "Default Key Commands" section of the Cubase Pro 14 Operation Manual on steinberg.help. That section has one page per ke... |
| `youtube_music` | YouTube Music | site | 21 | 1 | https://support.google.com/youtubemusic/answer/9092720, the URL Google's own community threads point to for this topic, returns a 404 ("t... |

### Office

| id | name | kind | actions | sources | note |
|---|---|---|---|---|---|
| `google_docs` | Google Docs | site | 192 | 1 | Current web editor, PC/ChromeOS and Mac columns. Where the vendor lists multiple alternate chords for one action, only the primary chord ... |
| `google_sheets` | Google Sheets | site | 118 | 1 | Current web editor, PC/ChromeOS and Mac columns. Where the vendor lists multiple alternate chords for one action, only the primary chord ... |
| `google_slides` | Google Slides | site | 169 | 1 | Current web editor, PC/ChromeOS and Mac columns. 'Video Player / Seek to a specific point' (Shift+0..Shift+9, jumping to 0%-90% of the cl... |
| `excel` | Microsoft Excel | app | 437 | 1 | From the Microsoft Support article "Keyboard shortcuts in Excel", Windows and macOS tabs. The Windows "Function keys" table packs several... |
| `onenote` | Microsoft OneNote | app | 241 | 1 | From the Microsoft Support article "Keyboard shortcuts in OneNote", Windows and macOS tabs (the desktop OneNote tabs; the OneNote for Win... |
| `outlook` | Microsoft Outlook | app | 553 | 2 | Windows shortcuts from the Microsoft Support article "Keyboard shortcuts for Outlook", both the Classic Outlook and New Outlook tabs (OUT... |
| `powerpoint` | Microsoft PowerPoint | app | 289 | 2 | From the two Microsoft Support articles "Use keyboard shortcuts to create PowerPoint presentations" and "Use keyboard shortcuts to delive... |
| `word` | Microsoft Word | app | 332 | 1 | From the Microsoft Support article "Keyboard shortcuts in Word", Windows and macOS tabs. Ribbon access keys documented as "Alt, H, F, O" ... |

### Other

| id | name | kind | actions | sources | note |
|---|---|---|---|---|---|
| `google_maps` | Google Maps | site | 7 | 1 | Source is Google Maps' accessibility help page, which documents only this short list (it also mentions pressing Ctrl+/ inside the map to ... |
| `wikipedia` | Wikipedia | site | 32 | 2 | These are MediaWiki access keys, not fixed chords: the underlying key is a single letter/punctuation mark, and the browser decides the mo... |

### Productivity

| id | name | kind | actions | sources | note |
|---|---|---|---|---|---|
| `onepassword` | 1Password | app | 70 | 1 | Covers 1Password 8 for Mac, Windows, and Linux. 'Switch accounts/collections: Ctrl/Cmd+2-9' and 'Rating'-style ranged shortcuts are numbe... |
| `acrobat` | Adobe Acrobat | app | 185 | 2 | Covers Acrobat on desktop (Reader and Pro), current version as of the Sep 23, 2025 update to Adobe's page (helpx.adobe.com/acrobat/using/... |
| `airtable` | Airtable | site | 54 | 1 | Not expressible, so left out: "Toggles extensions" (Ctrl+Shift / Cmd+Shift — the source documents no third key to complete the chord); th... |
| `asana` | Asana | site | 59 | 1 | The source article's "Desktop" section (New window, New tab, Close tab/window, Reopen closed tab, Next/Prev tab, Move tab left/right, Min... |
| `bitwarden` | Bitwarden | app | 28 | 1 | Covers the Bitwarden desktop app plus the browser-extension shortcuts the source page documents (same keys, listed separately by Bitwarde... |
| `clickup` | ClickUp | app | 114 | 2 | Covers both the ClickUp web app and the Windows/Mac desktop app; the shortcuts are the same keymap in both per the vendor's articles. The... |
| `clickup_web` | ClickUp (web) | site | 114 | 2 | twin of `clickup` |
| `confluence` | Confluence | site | 108 | 1 | Covers Confluence Cloud's in-app shortcuts (General/Content, Formatting, Table) and Whiteboards, as documented on the source page. 'Open ... |
| `evernote` | Evernote | app | 114 | 1 | Desktop app (Windows/Mac). "Global" context = system-wide hotkeys that fire whenever Evernote is running, even outside the app; all other... |
| `feedly` | Feedly | site | 25 | 1 | 'G then T/A/L/I/O/P' are two-key sequences (no modifier) per the chord grammar's sequence type. Not expressible, so left out: 'Show full ... |
| `google_calendar` | Google Calendar | site | 20 | 1 | Requires Settings > Enable keyboard shortcuts. The vendor also lists 'j' as an alternate for 'next date range' and 'w'/'1'/'2'/etc. as al... |
| `google_keep` | Google Keep | site | 24 | 1 | Mac column of the support article says the same functions apply with Cmd replacing Ctrl; that substitution is applied here to every Ctrl-... |
| `jira` | Jira | site | 11 | 3 | This is Jira Cloud (Atlassian cloud), the list shown in Jira's own "?" (Shift+/) shortcuts dialog: Global shortcuts, Navigating work item... |
| `linear` | Linear | app | 100 | 15 | Covers both the Linear web app and the Windows/Mac desktop app (same keymap per Linear's own docs). The full shortcut list otherwise live... |
| `linear_web` | Linear (web) | site | 100 | 15 | twin of `linear` |
| `apple_notes` | Notes | app | 52 | 1 | Mac only; Apple Notes has no Windows client. 'Create a Quick Note' (Fn+Q) is now included, since the chord grammar accepts Fn as a mac-on... |
| `notion` | Notion | app | 69 | 1 | Omitted as not chord-expressible: (1) text-wrapping markdown auto-format shortcuts (**bold**, *italic*, `code`, ~strikethrough~) which wr... |
| `notion_web` | Notion (web) | site | 69 | 1 | twin of `notion` |
| `obsidian` | Obsidian | app | 56 | 7 | Obsidian's own commands (open note, toggle sidebar, daily note, templates, backlinks, outline, etc.) are unbound by default and only get ... |
| `todoist` | Todoist | app | 88 | 1 | Covers the Todoist web app and Windows/Mac desktop apps (same keymap). Not expressible, so left out: Option/Alt+Click and Cmd/Ctrl+Click ... |
| `todoist_web` | Todoist (web) | site | 88 | 1 | twin of `todoist` |
| `trello` | Trello | site | 51 | 2 | "Bulk select cards" (Shift+Click) and "Scroll sideways" (Shift + mouse wheel) are mouse-dependent and not expressible in the chord gramma... |

### Shopping

| id | name | kind | actions | sources | note |
|---|---|---|---|---|---|
| `amazon` | Amazon.com | site | 7 | 1 | Not expressible in the chord grammar, so left out: opening the keyboard-accessible Navigation Assistant menu by pressing plain Tab after ... |

### Social

| id | name | kind | actions | sources | note |
|---|---|---|---|---|---|
| `facebook` | Facebook | site | 11 | 1 | The vendor documents scrolling between Feed stories as a single 'J and K' bullet without naming a direction; it is split here into two na... |
| `linkedin` | LinkedIn | site | 17 | 1 | Keyboard shortcuts are available in Desktop only (not iOS/Android, per the vendor FAQ). Contexts follow the vendor's own 'Location' colum... |
| `reddit` | Reddit | site | 25 | 1 | Covers the current (redesigned) reddit.com only; hotkeys aren't available on old Reddit. The vendor's 'Submit comment/post' row reads CMD... |
| `tiktok` | TikTok | site | 7 | 1 | TikTok's Help Center has no keyboard-shortcuts article: https://support.tiktok.com/en/using-tiktok/exploring-videos/keyboard-shortcuts (t... |
| `twitter_x` | X (Twitter) | site | 26 | 1 | The 'Timelines' section is documented as two-key sequences ('g' then a second letter), not modifier chords, so each is a sequence of two ... |

### System

| id | name | kind | actions | sources | note |
|---|---|---|---|---|---|
| `gnome` | GNOME | system | 56 | 2 | GNOME Shell defaults as shipped by Ubuntu. GNOME's Super key is written Win. Rows GNOME lists as Disabled by default are left out (Decrea... |
| `kde` | KDE Plasma | system | 76 | 2 | KDE Plasma defaults. KDE's Meta key is written Win. The Alt+D chords are two-step sequences, so they are recorded as type "sequence". Lef... |
| `macos` | macOS | system | 137 | 3 | Apple writes Command as Cmd, Option as Alt and Control as Ctrl here. The Mac Delete key (backwards delete) is written Backspace, which is... |
| `windows` | Windows | system | 143 | 2 | Windows 11 / Windows 10. File Explorer shortcuts live in catalog/windows_explorer.json. Left out because the chord grammar cannot express... |

### Video

| id | name | kind | actions | sources | note |
|---|---|---|---|---|---|
| `after_effects` | Adobe After Effects | app | 284 | 1 |  |
| `premiere` | Adobe Premiere Pro | app | 186 | 2 | Every row of Adobe's "Default keyboard shortcuts" page for Premiere (last updated Jan 7, 2026); helpx.adobe.com/premiere-pro/using/keyboa... |
| `resolve` | DaVinci Resolve | app | 177 | 2 | DaVinci Resolve 21. Blackmagic publishes no single consolidated shortcut list: the Keyboard Customization window inside the application i... |
| `disney_plus` | Disney+ | site | 15 | 1 | "Keyboard navigation" section of the Help Center article "Accessibility features on Disney+", covering DisneyPlus.com. Not expressible, s... |
| `final_cut_pro` | Final Cut Pro | app | 380 | 1 | Every row of Apple's "Keyboard shortcuts in Final Cut Pro for Mac" page. The action name is Apple's own command name (the Command column ... |
| `hulu` | Hulu | site | 16 | 1 | From the Help Center article "Using keyboard controls to navigate Hulu.com". The article states the Left/Right and Up/Down arrows each do... |
| `imovie` | iMovie | app | 64 | 1 | Every row of Apple's "Keyboard shortcuts in iMovie on Mac" page. That page names its commands only by what they do, so the action names a... |
| `mpv` | mpv | app | 109 | 1 | mpv's default bindings are documented per physical key with case treated as significant (an uppercase action requires Shift); bare letter... |
| `netflix` | Netflix | site | 9 | 1 | Official Help Center article documents 9 shortcuts total; 'Play / Pause' also accepts Enter (Space bar used here, matching the site's pri... |
| `obs_studio` | OBS Studio | app | 19 | 1 | The OBS Knowledge Base documents these fixed default shortcuts for editing sources and the preview window. Cropping/stretching/disabling ... |
| `plex` | Plex | app | 12 | 1 | Plex does not publish a dedicated 'Keyboard Shortcuts' support article for the Web App or the Windows/macOS desktop app. The only documen... |
| `plex_web` | Plex (web) | site | 12 | 1 | twin of `plex` |
| `prime_video` | Prime Video | site | 10 | 1 | From the Amazon Customer Service article "Keyboard Shortcuts on Prime Video", which covers Prime Video on a web browser (and the Windows/... |
| `nuke` | The Foundry Nuke | app | 185 | 1 |  |
| `twitch` | Twitch | site | 10 | 2 | Twitch's Help Center has no dedicated article documenting video-player keyboard shortcuts (searched help.twitch.tv for 'keyboard shortcut... |
| `vimeo` | Vimeo | site | 21 | 1 | The vendor table lists two separate keys for the same action in three cases (arrow keys and legacy YouTube-style letters both scrub/play)... |
| `vlc` | VLC media player | app | 126 | 1 | The VideoLAN wiki's Hotkeys table documents a single set of hotkeys for the Qt interface (Windows and Linux); it does not publish a separ... |
| `youtube` | YouTube | site | 49 | 2 | Not expressible in the chord grammar, so left out: Ctrl/Option+numpad zoom for 360° video, mouse-hover scrubbing, and playlist-only Shift... |

## Skipped targets

- **Claude.ai**: no official keyboard-shortcut article for claude.ai (only Claude Code is documented)
- **Gemini**: no official shortcut list for gemini.google.com
- **Google Photos**: no official article; only an undocumented in-app overlay
- **Pinterest**: no official list, only generic accessibility statements
- **Instagram**: the web app has no native keyboard shortcuts; third-party lists contradict each other (file removed)
- **Streamlabs Desktop**: hotkeys are user-assigned; no documented defaults
- **TIDAL**: no official shortcut article
- **Apple TV (web)**: no official article and the player needs a paid sign-in to probe
- **Epic Games Launcher**: Epic documents exactly one shortcut (Shift+F3 overlay); file removed as not useful

## Thin entries (vendor documents few shortcuts)

- `steam` (3): Steam Support does not publish a dedicated 'Steam client keyboard shortcuts' article (the FAQ id given in the task, 1B3B-6D0D-7C4D-8AC9, 404s); the three shortc
- `docker_desktop` (5): Docker Desktop has no dedicated keyboard-shortcut reference page. These are the only shortcuts Docker documents, drawn from its official release notes across ve
- `dropbox_web` (5): Web only (dropbox.com); the source states these shortcuts work only while viewing the Files tab of dropbox.com, and are the same on Windows and Mac. '?' (shown 
- `browser` (7): Combined browser profile used by the first-run config (Chromium-family chords). Per-browser entries chrome/edge/firefox/safari/brave/vivaldi carry the complete 
- `amazon` (7): Not expressible in the chord grammar, so left out: opening the keyboard-accessible Navigation Assistant menu by pressing plain Tab after page load (no modifier)
- `google_maps` (7): Source is Google Maps' accessibility help page, which documents only this short list (it also mentions pressing Ctrl+/ inside the map to reveal a live shortcut 
- `tiktok` (7): TikTok's Help Center has no keyboard-shortcuts article: https://support.tiktok.com/en/using-tiktok/exploring-videos/keyboard-shortcuts (the path named in the ta
- `foobar2000` (9): Windows only; there is no official macOS build. foobar2000's keyboard shortcuts are entirely user-assignable via Preferences > Keyboard Shortcuts, and the vendo
- `netflix` (9): Official Help Center article documents 9 shortcuts total; 'Play / Pause' also accepts Enter (Space bar used here, matching the site's primary/most common bindin
- `prime_video` (10): From the Amazon Customer Service article "Keyboard Shortcuts on Prime Video", which covers Prime Video on a web browser (and the Windows/macOS apps). The articl
- `twitch` (10): Twitch's Help Center has no dedicated article documenting video-player keyboard shortcuts (searched help.twitch.tv for 'keyboard shortcuts', 'hotkeys', and 'vid
- `facebook` (11): The vendor documents scrolling between Feed stories as a single 'J and K' bullet without naming a direction; it is split here into two named actions ('Scroll to
- `jira` (11): This is Jira Cloud (Atlassian cloud), the list shown in Jira's own "?" (Shift+/) shortcuts dialog: Global shortcuts, Navigating work items, and Work item action

## Caveats worth a human look

- **Fn shortcuts** (macOS Globe key) are included in the `mac` column. The engine can send them on macOS (Phase 5); Windows rejects them.
- **Task View / DisplayFusion** (added 2026-09-05): Task View is matched by explorer.exe plus the titles "Task View" / "Task Switching" (verified by a foreground probe). DisplayFusion's six default hotkeys are documented; its Alt+Tab handler was checked by hand: arrows work while Alt is held, and it also captures Ctrl+Alt+Tab, showing a stays-open popup that ignores arrows (disable its handler to get Windows' sticky switcher back).

- **Fn shortcuts** (macOS Globe key) are included in the `mac` column: Control Center Fn+C, Notification Center Fn+N, Quick Note Fn+Q, Dictation Fn+D, Show Desktop Fn+H, Fn+arrow navigation, and app-specific Fn alternates in Office, FL Studio and others. The engine can send them on macOS (Phase 5); Windows rejects them. Serif (Affinity) restructured its docs, so its Fn rows could not be re-sourced.

- **Capture One**: no default shortcut list is published anywhere; the 94 actions are every shortcut stated in prose across the help center.
- **SOLIDWORKS**: only the partial list SOLIDWORKS publishes; the full default set is only printable from Tools > Customize in-product.
- **Xcode**: the only complete published list is the Xcode 4-era developer-library archive; current Xcode may differ.
- **REAPER**: from the official v2.42 default-shortcuts summary, cross-checked against the v7.79 user guide where it prints keys.
- **DaVinci Resolve**: Blackmagic publishes no consolidated list; Edit-page tables from the reference manual; Color/Fusion/Fairlight/Deliver pages absent.
- **ChatGPT**: the help-center article was retired; taken from the current commands reference and filtered to web-relevant items (lower confidence).
- **TikTok**: no help article; bindings confirmed by testing the live player.
- **Hulu**: the article does not state volume direction; standard Up/Down = volume assumed (noted in file).
- **Reddit**: submit is Alt+Enter on Windows because the vendor says its Cmd auto-translates to Alt.
- **Photoshop**: Adobe retired the shortcuts page; merged from two Wayback captures on top of the ShortcutMapper import.
- **Vim / Emacs / tmux**: capital-letter commands are encoded as Shift+letter; prefix chords as sequences; commands taking an argument are omitted by design.
- **Web twins** (`slack_web`, `notion_web`, `discord_web`, `spotify_web`, `figma_web`, `linear_web`, `clickup_web`, `miro_web`, `plex_web`, `todoist_web`) share the desktop entry's actions; browser-only differences are not captured.

## Process

Four authoring waves, 21 authoring agents (Opus for sprawling, multi-source targets; Sonnet for single-page targets) plus four Sonnet QA agents. Agents wrote only their own `catalog/<id>.json` files; all generator, validator, test and UI code changes were made in the main session. Some agents ran a read-only `git status`; no agent changed repository state.

