Output "media/ani-cli-flow.gif"
Set Width 2560
Set Height 1440
Set Framerate 30
Set FontSize 20
Set CursorBlink true

Type "ani-cli"
Enter
Sleep 2s

# Open search and query
Type "/"
Sleep 200ms
Type "steel ball run"
Sleep 1s
Screenshot "media/search.png"
Enter

# Episode detail
Sleep 1s
Screenshot "media/episode-detail.png"

# Play episode 1
Enter
Sleep 1s
Screenshot "media/now-playing.png"

Sleep 2s
Type "Q"
