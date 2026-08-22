_vj_completions() {
    local cur prev words cword
    _init_completion || return

    local commands="record play preview list random encrypt decrypt stats profiles config import inbox-server preview-inbox completions help"
    local profiles="potato compact terry balanced hq"

    if [[ $cword -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "${commands}" -- "${cur}") )
        return 0
    fi

    case "${words[1]}" in
        record)
            if [[ "${prev}" == "-p" || "${prev}" == "--profile" ]]; then
                COMPREPLY=( $(compgen -W "${profiles}" -- "${cur}") )
            else
                COMPREPLY=( $(compgen -W "-p --profile -e --encrypt --no-encrypt -t --title --tags -n --note -i --interactive -v --verbose ${profiles}" -- "${cur}") )
            fi
            ;;
        import)
            if [[ "${prev}" == "-p" || "${prev}" == "--profile" ]]; then
                COMPREPLY=( $(compgen -W "${profiles}" -- "${cur}") )
            else
                COMPREPLY=( $(compgen -W "-p --profile -e --encrypt --no-encrypt -t --title --tags -n --note -i --interactive --keep -v --verbose ${profiles}" -- "${cur}") )
            fi
            ;;
        play|preview)
            local entries
            entries=$(vj list -q 2>/dev/null)
            COMPREPLY=( $(compgen -W "${entries}" -- "${cur}") )
            ;;
        encrypt|decrypt)
            local entries
            entries="all $(vj list -q 2>/dev/null)"
            COMPREPLY=( $(compgen -W "${entries}" -- "${cur}") )
            ;;
        completions)
            COMPREPLY=( $(compgen -W "bash zsh fish install" -- "${cur}") )
            ;;
        list|ls)
            COMPREPLY=( $(compgen -W "-q --quiet" -- "${cur}") )
            ;;
    esac
}
complete -F _vj_completions vj
