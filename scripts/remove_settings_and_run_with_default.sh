#!/usr/bin/env bash

SETTINGS_FILE=~/.local/share/logviewer/app.ron
if [ -e $SETTINGS_FILE ]
then 
    trash $SETTINGS_FILE
    echo "Trashed settings file before run"
else
    echo "Settings file not found, so not deleted."
fi 
cargo run