# Hera
[![Linux build status](https://img.shields.io/travis/XAMPPRocky/hera.svg?branch=master)](https://travis-ci.org/XAMPPRocky/hera)
[![Windows build status](https://ci.appveyor.com/api/projects/status/github/XAMPPRocky/hera?svg=true)](https://ci.appveyor.com/project/XAMPPRocky/hera)
[![](https://img.shields.io/crates/d/hera.svg)](https://crates.io/crates/hera)
[![](https://img.shields.io/github/issues-raw/XAMPPRocky/hera.svg)](https://github.com/XAMPPRocky/hera/issues)
[![](https://tokei.rs/b1/github/XAMPPRocky/hera?category=code)](https://github.com/XAMPPRocky/hera)
[![Documentation](https://docs.rs/hera/badge.svg)](https://docs.rs/hera/)

Hera checks if there were actual code changes in the last commit in git
repositories. Allowing you to skip building your project if only documentation
or comments have changed.  This is mainly useful for projects that have really
long build times. Hera supports all languages supported by [tokei].

## Installation

#### Binary

###### Automatic
```
cargo install hera
```

###### Manual
You can download prebuilt binaries in the [releases section] or create one
from source.

```shell
$ git clone https://github.com/XAMPPRocky/hera.git
$ cd hera
$ cargo build --release
```
###### Linux/OSX
```
# sudo mv target/release/hera /usr/local/bin
```
###### Windows
- Create a folder for hera
- search for `env`
- open "edit your enviroment variables"
- edit `PATH`
- append folder path to the end of the string ie: `<path>;C:/hera/;`

## Help
```
hera 0.1.0
Aaron P. <theaaronepower@gmail.com> + Contributors
A program for checking if there were code changes between git commits.

USAGE:
    hera [FLAGS] [OPTIONS] [input]...

FLAGS:
    -h, --help       Prints help information
    -q, --quiet      Do not output to stdout.
    -V, --version    Prints version information

OPTIONS:
    -f, --filter <filter>    Filters by language, seperated by a comma. i.e. -t=Rust,C

ARGS:
    <input>...    The git repositories to be checked. Defaults to the current directory.
```

## Example

```
# Run cargo build if there were code changes
hera && cargo build
```

[releases section]: https://github.com/XAMPPRocky/hera/releases
[tokei]: https://github.com/XAMPPRocky/tokei

