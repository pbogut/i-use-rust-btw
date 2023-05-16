# i3-focus

Command that can switch focus between i3 windows as well as Neovim and Tmux
splits. It just works.

## i3 configuration

```
# i3/config
set $focus exec --no-startup-id i3-focus
bindsym $mod+h $focus left
bindsym $mod+j $focus down
bindsym $mod+k $focus up
bindsym $mod+l $focus right
```

## Neovim configuration

Window title needs to match 

```regex
/^[^:]:nvim:(\d+):.*$/`
```

For example `"user@host:nvim:12345:file"`

```lua
-- init.lua
local user = os.getenv('USER') .. ''
local host = fn.hostname()
local nvim_addr = vim.v.servername or ''
local addr = fn.substitute(nvim_addr, [[/run/user/[0-9]\+/nvim\.\(.*\)\.0]], [[\1]], 'g')
vim.o.titlestring = user .. '@' .. host .. ':nvim:' .. addr .. ':' .. vim.fn.getcwd()
```


## Tmux configuration

Window title needs to match 
```regex
/.* |t\$(\d+)$/ 
```
For example `"My Title |t$45"`

```sh
# tmux.conf
set -g set-titles on
set -g set-titles-string "#T |t#{session_id}"
```
