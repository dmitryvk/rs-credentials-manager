init-tmp:
    mkdir -p tmp
    if ! [ -f tmp/.gitignore ]; then echo '*' > tmp/.gitignore; fi

run-tui: init-tmp
    cargo run -p cred-man-tui -- tmp
