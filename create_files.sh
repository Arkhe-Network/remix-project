cat user_prompt.txt | grep -E '^# ' | awk '{print $2}' > files.txt
