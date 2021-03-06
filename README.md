# yanker

A Rust crate to automate crate-yanking.

## Usage
Install via cargo-install:
```
$ cargo install yanker
```

Change directories to the crate you want to yank:
```
$ cd path/to/crate/repo
$ yanker "[0.1.1, 0.2.5]"
```
Should yank all versions between 0.1.1 (included) to 0.2.5 (excluded).