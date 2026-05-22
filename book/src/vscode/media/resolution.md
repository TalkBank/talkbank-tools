# Media Resolution

**Status:** Current
**Last updated:** 2026-05-21 13:25 EDT

This chapter explains how the extension locates media files referenced by CHAT
transcripts. Understanding the resolution process helps when media playback fails
or when organizing corpus files on disk.

## The @Media Header

Every `.cha` file that uses media features must include an `@Media:` header
specifying the base filename and the media type:

```text
@Media: interview, audio
@Media: session03, video
```

The header provides two pieces of information:

1. **Base filename** --- the name without extension (e.g., `interview`)
2. **Media type tag** --- either `audio` or `video`. Downstream features
   may use this label, but the file-resolution step does **not** branch
   on it — the resolver uses a single ordered list of extensions
   regardless of the declared type.

The extension list lives in `vscode/src/utils/mediaResolver.ts`
(`MEDIA_EXTENSIONS`), ordered by commonality:

`.mov`, `.mp4`, `.mp3`, `.wav`, `.m4v`, `.aif`, `.avi`, `.wmv`,
`.mpg`, `.aiff`.

## Resolution Search Order

When you trigger playback or open the waveform, the resolver in
`vscode/src/utils/mediaResolver.ts::resolveMediaPath` searches in this
order:

1. **Verbatim** --- if the `@Media:` value already includes an
   extension that exists on disk, that exact path wins.
2. **Same directory, every extension** --- for each `.ext` in
   `MEDIA_EXTENSIONS`, try `docDir/<stem><ext>`.
3. **`media/` subdirectory, every extension** --- for each `.ext` in
   `MEDIA_EXTENSIONS`, try `docDir/media/<stem><ext>` (CLAN's
   conventional layout).

The first existing file wins. There are no additional configured search
roots — the resolver only looks in the document's own directory and a
sibling `media/`.

### Example

Given a file at `/corpus/English/Brown/Adam/adam01.cha` with header
`@Media: adam01, audio`, the resolver searches in order:

```text
/corpus/English/Brown/Adam/adam01.mov
/corpus/English/Brown/Adam/adam01.mp4
/corpus/English/Brown/Adam/adam01.mp3
/corpus/English/Brown/Adam/adam01.wav
/corpus/English/Brown/Adam/adam01.m4v
  ... (the rest of MEDIA_EXTENSIONS in order)
/corpus/English/Brown/Adam/media/adam01.mov
/corpus/English/Brown/Adam/media/adam01.mp4
  ...
```

## Document Links

`@Media` file references are rendered as **clickable links** in the editor. If the
resolver finds the media file, clicking the link opens it directly in the system's
default media player (or in VS Code's built-in preview for supported formats).

This provides a quick way to verify that the media file exists and is accessible
without starting playback mode.

## Picture Display

The extension can display elicitation pictures (Cookie Theft, picture descriptions,
etc.) alongside the transcript --- the CLAN equivalent of PictController.

### Opening a Picture

1. Open a `.cha` file
2. Right-click in the editor and select **TalkBank: Media** then **Show Elicitation
   Picture**, or use the Command Palette

### How Pictures Are Found

The extension searches for associated images using three strategies, in order:

1. **`%pic:` tier references** --- if the document contains dependent tier lines like
   `%pic: "image002.jpg"`, those filenames are used directly

2. **Same-name image files** --- image files matching the `.cha` file's base name
   (e.g., `adam01.cha` matches `adam01.jpg`, `adam01.png`)

3. **Directory contents** --- any image files in the same directory as the `.cha` file

### Multi-Image Picker

If multiple candidate images are found, a picker dialog appears listing all matches.
Select the desired image to display it.

### Display

The selected picture opens in a side panel and scales to fit the available space.
You can arrange the picture panel alongside the transcript and waveform for a
complete annotation workspace.

## Troubleshooting

When media playback or the waveform panel fails to open, check the following:

### 1. Verify the @Media Header

The `.cha` file must contain a valid `@Media:` line. Check for:

- Missing header entirely --- add `@Media: filename, audio` (or `video`)
- Typos in the filename --- the base name must match the actual media file
- Wrong media type --- `audio` vs `video` must match the actual file format

### 2. Check That the File Exists

Verify that the media file is present in one of the resolver's search locations
(same directory or `media/` subdirectory). The filename must match the `@Media`
base name exactly (case-sensitive on macOS and Linux).

### 3. Check the File Extension

The media file must have one of the recognized extensions in
`MEDIA_EXTENSIONS` (above). The resolver does not distinguish audio
from video at the search step — an `.mp3` file declared as `video` in
the `@Media` header will still be found, because the same extension
list is tried for every header. If the actual file uses an extension
not in that list, add the extension explicitly to the `@Media:` value
(e.g., `@Media: adam01.flac, audio`), which makes the resolver hit the
verbatim path first.

### 4. Reproduce the search by hand

The resolver in `vscode/src/utils/mediaResolver.ts` does not currently
emit log output for each attempted path. If a file should match but
doesn't, walk through the candidate list from "Resolution Search Order"
above by hand: list the directory contents (`ls /path/to/.cha/dir/`
and `ls /path/to/.cha/dir/media/` if present) and look for the stem
with any of the extensions in `MEDIA_EXTENSIONS`. The most common
causes are filename case mismatch on macOS / Linux, a stray extension
not in the list (e.g., `.ogg`, `.flac`, `.mkv`), or an unexpected
extra directory level between the `.cha` and the media.

### 5. Large or Unsupported Formats

VS Code's embedded browser engine (Chromium) has limits on media format support.
If the file exists but playback fails, the format may not be supported. Common
working formats are MP3, WAV, and MP4 (H.264). Uncommon codecs (e.g., FLAC in
some configurations, AVI with legacy codecs) may not play.

## See Also

- [Media Playback](playback.md) --- playback controls and behavior
- [Waveform Visualization](waveform.md) --- visual audio waveform
- [Media Troubleshooting](../troubleshooting/media.md) --- extended troubleshooting guide
