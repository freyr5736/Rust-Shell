# Rust Shell

A simple shell implemented in Rust, simulating basic shell functionalities such as executing built-in commands (`cd`, `pwd`), handling external commands, and supporting output and error redirection.

## Features

- **Built-in Commands**:
  - `cd`: Change the current directory.
  - `pwd`: Print the current working directory.

- **External Command Execution**: 
  - Supports running external commands that exist in the system's `PATH`.

- **Redirection**:
  - Supports output redirection (`>`, `>>`).
  - Supports error redirection (`2>`, `2>>`).

## Prerequisites

Before running the project, ensure you have the following installed:

- **Rust**: You can install Rust by following the instructions at [https://www.rust-lang.org/](https://www.rust-lang.org/).

- **Git**: You need Git to clone the repository and version control.

## Installation

1. **Clone the Repository**:

   ```bash
   git clone https://github.com/freyr5736/Rust-Shell.git
   cd Rust-Shell


## Usage
Once the shell is running, you can use the following commands:

pwd: Prints the current working directory.

Example:

```bash
$ pwd
/home/user
cd <dir>: Changes the current directory to the specified one.

Example:

```
$ cd /path/to/directory


Redirection: You can redirect output or error to a file using the following operators:

```
>: Redirects standard output to a file.
>>: Appends standard output to a file.
2>: Redirects standard error to a file.
2>>: Appends standard error to a file.

Example:

```bash
$ echo "Hello, World!" > output.txt
This will create (or overwrite) the output.txt file with the output of the echo command.
