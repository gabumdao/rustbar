# bash completion for rustbarctl
_rustbarctl_complete() {
    local cur prev
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    local commands="reload refresh fullscreen show hide toggle status list-outputs complete-outputs quit help"
    local global_flags="--socket -s --help -h"

    if [[ "$prev" == "--socket" || "$prev" == "-s" ]]; then
        compopt -o filenames 2>/dev/null
        COMPREPLY=( $(compgen -f -- "$cur") )
        return 0
    fi

    local command=""
    for word in "${COMP_WORDS[@]:1}"; do
        case "$word" in
            --socket|-s)
                break
                ;;
            reload|refresh|fullscreen|show|hide|toggle|status|list-outputs|complete-outputs|quit|help)
                command="$word"
                break
                ;;
        esac
    done

    if [[ -z "$command" ]]; then
        COMPREPLY=( $(compgen -W "$global_flags $commands" -- "$cur") )
        return 0
    fi

    case "$command" in
        show|hide|toggle|status)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--json" -- "$cur") )
            else
                COMPREPLY=( $(compgen -W "$(rustbarctl complete-outputs "$cur" 2>/dev/null)" -- "$cur") )
            fi
            ;;
        reload|refresh|fullscreen|list-outputs)
            COMPREPLY=( $(compgen -W "--json" -- "$cur") )
            ;;
        complete-outputs)
            COMPREPLY=( $(compgen -W "$(rustbarctl complete-outputs "$cur" 2>/dev/null)" -- "$cur") )
            ;;
        quit|help)
            ;;
    esac
}

complete -F _rustbarctl_complete rustbarctl
