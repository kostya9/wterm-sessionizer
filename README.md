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
cargo install wterm-sessionizer --version 0.0.13-alpha
```

Add to your powershell profile

```pwsh
@((wts init) -join "`n") | Invoke-Expression
```
