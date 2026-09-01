# meshmesh
messaging app so we can get off discord

## Development

```
cargo run
```

If u are using the GUI:
```
cargo run --features gui
```

### Development on WSL
Ensure you have libfontconfig-dev or else it won't build. Otherwise just run
```
sudo apt install libfontconfig-dev
```
Remember that on WSL2 there generally aren't any display servers (unless u have wslg), so u can only use the terminal view for development
