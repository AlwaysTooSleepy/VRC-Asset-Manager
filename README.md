# VRChat Asset Manager

A desktop app for organizing your VRChat avatar models and the assets that go
with them — clothing, accessories, shaders, and anything else you collect.

> **Built with AI** — See [AI_DISCLOSURE.md](AI_DISCLOSURE.md) for details.

---

## What does it do?

If you buy VRChat avatars and assets from stores like Booth, Gumroad, Jinxxy,
or Payhip, you probably end up with a mess of downloaded folders spread across
your computer. This app gives them a home.

**VRChat Asset Manager lets you:**

- 📁 **Organize** models and assets into a clean, consistent folder structure
- 🖼️ **Attach images** as thumbnails so you can see what everything looks like
- 🔗 **Track where you bought things** with source URLs
- 📦 **Manage versions** so you always know which update you have
- 🔍 **Filter and search** by creator, category, or compatibility
- 🌐 **Auto-fill item details** by pasting a Booth, Gumroad, Jinxxy, or Payhip
  product URL — the app fetches the name, creator, and images for you

---

## How your files are organized

When you first launch the app, you choose a folder to use as your **library
root**. The app creates this structure inside it:

```
MyVRChatLibrary/
├── Models/
│   └── CreatorName/
│       └── AvatarName_V_1.0/      ← the avatar's files live here
├── Assets/
│   └── CategoryName/
│       └── CreatorName/
│           └── AssetName_V_1.0/   ← the asset's files live here
└── AppData/
    ├── Images/                    ← thumbnail images (copies, originals untouched)
    └── SQLite/
        └── library.sqlite         ← the live database
```

When you add a model or asset and point it at a source folder, the app
**moves** that folder into the library structure. If the source is on a
different drive than the library, it copies the files and then removes the
originals.

---

## Download and install

> **Pre-built installers are not yet available.**
>
> To run the app, you need to build it from source. See
> [CONTRIBUTING.md](CONTRIBUTING.md) for step-by-step setup instructions.

---

## Basic usage walkthrough

### 1. Choose your library folder

On first launch, the app asks you to pick or create a folder. This is where
all your organized files will live. Pick somewhere with plenty of space.

### 2. Add a model (avatar)

Click **Add Model**. You can either:
- **Paste a store URL** (Booth, Gumroad, Jinxxy, or Payhip) and let the app
  fill in the name, creator, and images automatically, or
- Fill in the details manually.

Then choose the folder that contains the avatar files. The app moves it into
`Models/CreatorName/AvatarName_V_1.0/`.

### 3. Add an asset

Click **Add Asset**. Same process as a model — paste a URL or fill in
manually. You can mark which avatar(s) the asset is compatible with, or mark
it as compatible with all.

### 4. Browse and filter

The home screen shows all your models and assets. Use the filters at the top
to narrow down by creator, category, or compatibility.

### 5. Open files in Explorer / Finder

Click any model or asset to open its detail view, then click **Open Folder**
to jump straight to its files on disk.

---

## Keeping your library safe

**The most important rule: back up your entire library root folder.**

The database (`library.sqlite`) and your files live together. Backing up one
without the other is not a complete backup.

**Recommended backup routine:**

1. Close the app.
2. Copy your entire library root folder to an external drive or cloud storage.

---

## Settings

Open **Settings** (gear icon, top right) to manage:

| Setting | What it does |
|---|---|
| **Sites** | Add or rename the stores you buy from (Booth, Gumroad, etc.) |
| **Creators** | Add, edit, or remove creators. Renaming a creator also renames their folder on disk. |
| **Categories** | Add, edit, or remove asset categories. Renaming also renames folders. |
| **Library** | Change your library folder or reconcile metadata with what's on disk. |

---

## Reconcile (sync metadata with disk)

If you rename or move files outside the app, the metadata can get out of sync.
Go to **Settings → Library → Reconcile** to compare what the database knows
about with what's actually on disk, and resolve any differences.

---

## FAQ

**Can I move my library to a new location?**  
Yes. Move the entire library root folder to the new location, then open the
app and use **Settings → Library → Change folder** to point it at the new
path.

**Does the app extract ZIP files?**  
No. Extract your downloads manually first, then point the app at the extracted
folder.

**What happens if I delete an item in the app?**  
The metadata entry is removed from the database. The files on disk are **not**
deleted. You will need to remove them manually if you want to free up space.

**Can I use the library on multiple computers?**  
The library root is portable — library-relative paths are used internally, so
you can copy it to another machine. Open the app and choose the copied folder
as your library. Note that if two computers modify the library at the same
time, the results will conflict.

**What stores does auto-fill support?**  
Booth, Gumroad, Jinxxy, and Payhip. For other stores, fill in the details
manually.

---

## For developers

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions, project
structure, and how to submit changes.

---

## License

[MIT License](LICENSE) — free to use, modify, and distribute.

---

## AI Disclosure

This project was built with the assistance of AI. See
[AI_DISCLOSURE.md](AI_DISCLOSURE.md).
=======
# VRChatAssetManager
An application to help organize assets associated with VRChat, e.g. Models, Avatar Systems, Worlds. 
>>>>>>> 73ea90eebed05f4718b6e026290b13384fe84703
