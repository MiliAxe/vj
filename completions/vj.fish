# Fish completion for vj

set -l commands record play preview list random encrypt decrypt stats config completions help

complete -c vj -f
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a record -d "Record a new video entry"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a play -d "Play entry in mpv (RAM streaming for encrypted)"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a preview -d "Preview entry metadata and notes"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a list -d "List all entries in table format"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a random -d "Play a random past entry"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a encrypt -d "Encrypt entry with GPG AES-256"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a decrypt -d "Decrypt entry to plaintext"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a stats -d "Show summary stats and storage"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a config -d "Edit configuration file"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a completions -d "Output or install shell completions"

# Record options
complete -c vj -n "__fish_seen_subcommand_from record" -s p -l profile -x -a "terry balanced hq" -d "Compression profile"
complete -c vj -n "__fish_seen_subcommand_from record" -s e -l encrypt -d "Encrypt with GPG AES-256"
complete -c vj -n "__fish_seen_subcommand_from record" -l no-encrypt -d "Save unencrypted"
complete -c vj -n "__fish_seen_subcommand_from record" -s t -l title -r -d "Title of entry"
complete -c vj -n "__fish_seen_subcommand_from record" -l tags -r -d "Comma-separated tags"
complete -c vj -n "__fish_seen_subcommand_from record" -s n -l note -d "Open editor for notes"
complete -c vj -n "__fish_seen_subcommand_from record" -s i -l interactive -d "Prompt for title and note"
complete -c vj -n "__fish_seen_subcommand_from record" -s v -l verbose -d "Verbose ffmpeg/mpv output"

# Subcommands taking entry IDs
function __fish_vj_entries
    vj list -q 2>/dev/null
end

complete -c vj -n "__fish_seen_subcommand_from play preview" -a "(__fish_vj_entries)" -d "Entry ID"
complete -c vj -n "__fish_seen_subcommand_from encrypt" -a "all (__fish_vj_entries)" -d "Entry ID or 'all'"
complete -c vj -n "__fish_seen_subcommand_from decrypt" -a "all (__fish_vj_entries)" -d "Entry ID or 'all'"
complete -c vj -n "__fish_seen_subcommand_from completions" -a "fish bash zsh install" -d "Shell type or install"
