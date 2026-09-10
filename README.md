# meshmesh
messaging app so we can get off discord

## Development

```
cargo run --bin cli
```

If u are using the GUI, get the dioxus-cli either by cargo-binstall or with --locked:
```
cargo install cargo-binstall    
cargo binstall dioxus-cli --force

// or

cargo install dioxus-cli --locked 
```

And then with dioxus-cli:
```
dx serve --package gui --platform desktop
```

### Development on WSL
Ensure you have libfontconfig-dev or else it won't build. Otherwise just run
```
sudo apt install libfontconfig-dev
```
Remember that on WSL2 there generally aren't any display servers (unless u have wslg), so u can only use the terminal view for development
