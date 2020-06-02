Changelog
=========

## v0.3.0

### Breaking Changes
*  `lines:` and `line:` commands now uses 1-based index, and line range is now inclusive:
    * `file.txt | lines: 1 3` returns the 1st, 2nd and 3rd lines of *file.txt*.
    * `file.txt | line: 1 3` returns the 1st and 3rd lines of *file.txt*.
* Removed `before:` and `after:` commands.
    * `before: "("` can be replaced with can just use `wrap: "(" ""`
    * `after: ")"` can be replaced with can just use `wrap: "" ")"`
    
    
### New Features
* Added `wrap-lines:` command to wrap each line instead of the whole file.
    * `alphabet.txt | wrap-lines: "-> " "!"` generates:
    ```
    -> A!
    -> B!
    -> C!
    -> D!
    ```
* Added `line-numbers:` command to prefix each line with its line number (starts counting from 1).
    * Takes optional separator as an argument
* Added support for `depend_dirs` in the `.md-inc.toml` config file.
    * These are processed before the current directory.
* Added `out_dir` as CLI argument and config field.
    * Redirects output instead of overwriting input file.
    
### Updates
* Updated README, fixed incorrect examples and typos.
* Reorganised example code and made generation in `doc` cleaner.