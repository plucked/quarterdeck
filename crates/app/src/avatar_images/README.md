# avatar_images

People's GitHub avatars, drawn round beside their names. A view asks the global store for the avatars it is about to show; the store downloads each once per session in the background (through `qd-github`, from GitHub's public avatar host, without the token) and decodes it for gpui, and views observing it redraw as avatars arrive. Until then, and when one cannot be loaded, a quiet circle stands in. Avatars are not written to the database.
