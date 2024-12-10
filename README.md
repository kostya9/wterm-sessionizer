# This is a simple plugin that automates finding GIT projects in the system.

## Usage
wts {path=optional}

Now you can search through the repositories traversable from the path you launched the program.
```
? Select repository > 
c:\repo1 [js]
c:\repo2 [csharp]
```

## Installation:
```
cargo install wterm-sessionizer --version 0.0.25-alpha
```

Add to your powershell profile

```pwsh
@((wts init) -join "`n") | Invoke-Expression
```

> [!IMPORTANT]
> Add the powershell profile code AFTER the prompt modifiers like oh-my-posh so that they won't overwrite the custom logic of wts.

--- 

Adds a "wcd" function that tracks what directories you open usually, and can be used to quickly navigate to them.

E.g. you usually go to a directory called "c:\repo1\src\frontend" and "c:\repo1\src\backend" you can use wcd to quickly navigate to them.

```pwsh
wcd repo1

wcd frontend

wcd backend

```
