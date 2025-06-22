# PixelShot
`PixelShot` is a utility for taking screenshots and potentially editing them (linux)

### build instructions:

    git clone https://github.com/awaprim/PixelShot.git
    cd PixelShot
    cargo build --release

### usage:
take copy to clipboard:

    pixelshot

with image editor:

    pixelshot --editor


### editor keybinds:
`ctrl+c` -> copy to clipboard
`ctrl+z` -> undo last change
`ctrl+shift+z` -> redo last change (scuffed)
