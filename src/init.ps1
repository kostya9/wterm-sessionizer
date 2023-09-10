
$oldPrompt = (Get-Command prompt).ScriptBlock 

$newPrompt = "$oldPrompt; `$newDir = Get-Location; if (`$newDir -ne `$oldDir) {wts on-changed-directory}; `$oldDir = `$newDir"

Set-Item -Path Function:prompt -Value $newPrompt

function wts { @(wts $args) }
