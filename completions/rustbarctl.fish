complete -c rustbarctl -f

complete -c rustbarctl -s s -l socket -r -d 'Override the control socket path'
complete -c rustbarctl -s h -l help -d 'Show help'

complete -c rustbarctl -n 'not __fish_seen_subcommand_from reload refresh fullscreen show hide toggle status list-outputs complete-outputs quit help' -a reload -d 'Reload the active config'
complete -c rustbarctl -n 'not __fish_seen_subcommand_from reload refresh fullscreen show hide toggle status list-outputs complete-outputs quit help' -a refresh -d 'Redraw all active surfaces'
complete -c rustbarctl -n 'not __fish_seen_subcommand_from reload refresh fullscreen show hide toggle status list-outputs complete-outputs quit help' -a fullscreen -d 'Rerun the fullscreen-state check'
complete -c rustbarctl -n 'not __fish_seen_subcommand_from reload refresh fullscreen show hide toggle status list-outputs complete-outputs quit help' -a show -d 'Show all rustbar surfaces or one output'
complete -c rustbarctl -n 'not __fish_seen_subcommand_from reload refresh fullscreen show hide toggle status list-outputs complete-outputs quit help' -a hide -d 'Hide all rustbar surfaces or one output'
complete -c rustbarctl -n 'not __fish_seen_subcommand_from reload refresh fullscreen show hide toggle status list-outputs complete-outputs quit help' -a toggle -d 'Toggle visibility for all rustbar surfaces or one output'
complete -c rustbarctl -n 'not __fish_seen_subcommand_from reload refresh fullscreen show hide toggle status list-outputs complete-outputs quit help' -a status -d 'Print status for all outputs or one output'
complete -c rustbarctl -n 'not __fish_seen_subcommand_from reload refresh fullscreen show hide toggle status list-outputs complete-outputs quit help' -a list-outputs -d 'List active outputs'
complete -c rustbarctl -n 'not __fish_seen_subcommand_from reload refresh fullscreen show hide toggle status list-outputs complete-outputs quit help' -a complete-outputs -d 'Print active output names'
complete -c rustbarctl -n 'not __fish_seen_subcommand_from reload refresh fullscreen show hide toggle status list-outputs complete-outputs quit help' -a quit -d 'Stop rustbar cleanly'
complete -c rustbarctl -n 'not __fish_seen_subcommand_from reload refresh fullscreen show hide toggle status list-outputs complete-outputs quit help' -a help -d 'Show help'

complete -c rustbarctl -n '__fish_seen_subcommand_from reload refresh fullscreen list-outputs' -l json -d 'Machine-readable output'
complete -c rustbarctl -n '__fish_seen_subcommand_from show hide toggle status' -l json -d 'Machine-readable output'
complete -c rustbarctl -n '__fish_seen_subcommand_from show hide toggle status complete-outputs' -a '(rustbarctl complete-outputs 2>/dev/null)'
