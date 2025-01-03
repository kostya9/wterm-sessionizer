$function:prompt = & {
	$__last_prompt = $function:prompt
	{ & $script:__last_prompt
		$newDir = Get-Location
		if ($newDir -ne $global:oldDir) {wts on-changed-directory $newDir}
		$global:oldDir = $newDir
	}.GetNewClosure()
}

function wts { $result = @((wts.exe $args) -join "`n"); if ($result.StartsWith("<#Execute#>")) { @($result) | Invoke-Expression } else { echo $result } }

function wcd { $result = @((wts.exe expand-cd $args) -join "`n"); if ($result.StartsWith("<#Execute#>")) { @($result) | Invoke-Expression } else { echo $result }  }
