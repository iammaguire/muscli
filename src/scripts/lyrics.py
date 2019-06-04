import lyricsgenius as genius
api = genius.Genius("ZQwvKeOrcoiQ2Mt1m5g6M0pB-344NPbN0s1P15ETH91uPVBrf6OWl_xI8CUDRggE")
artist = api.search_song("Run It", "Logic")
print(artist.lyrics)
