###### i do not know how to name this

# rhythm_typer_editor

This is a minimal beatmap editor for the rhythm game [Rhythm Typer](https://rhythmtyper.net/). This is very much work in progress.

## Why?

I am very used to using my mouse to place and click to place notes. Therefore this aims to implement this in a desktop app.

## (Added) Features

- Load `.rtm` files.
- Edit multiple difficulties by switching between them.
- Scrollable timeline (like in osu!).
- Edit beatmaps by clicking the keys (like placing hitcircles in osu!).
- Automatic note snapping.
- Very intuitive and easy creation of hold notes.
- Edit basic metadata (but not all, yet).

## Todo List

- Timing Point editors.
- Background Image and Background Video display.
- Complete workflow to create beatmap from scratch

## Creating a beatmap from scratch

Currently this project does not allow you to do this, you must start with an already made `.rtm` file. 

To create a beatmap from scratch, use the [Rhythm Typer](https://rhythmtyper.net/) Editor to create a `.rtm` beatmap with only one object, when or where does not matter, as you can delete the note later.

Import this `.rtm` file into the app. Start creating!

## Using the editor

This section will attempt to give a brief walkthrough of how to use the editor; I hope you will find it quite intuitive.

### Importing Maps and Adding Difficulties

- **Importing Mapsets:** Use the Load `.rtm` button to import a mapset into the editor.

- **Selecting Difficulties:** To the left of the Load `.rtm` button is the difficulty selector. Choose the difficulty you'd like to edit here.

- **Importing Difficulties:** The import difficulty button imports a JSON difficulty. 

- **Exporting Difficulties:** The export difficulty button exports the difficulty as a JSON.

- **Saving Your Work:** The save `.rtm` button overwrites the `.rtm` file that was imported with the new data.

### Editing the beatmap

- **Adding Notes:** Use the mouse to left click the keys on the keyboard to add a key to the beatmap. A preview is given on the top timeline.

- **Removing Notes:** Use the mouse to right click the keys on the keyboard to remove a key. This requires moving the current time to the time of the note.

- **Toggle between Tap and Hold:** Use `CAPSLOCK` to toggle modes between Tap notes and Hold notes. 

- **Hold Notes:** Hold notes follow a toggle system. Press a key to start the hold note, and then click it again when you'd like to end it and any other given time.

### Navigation

- **Zooping Through the Map:** Use the scroll wheel (or trackpad) to scroll through the timeline. This will automatically adjust the audio also.

- **Pause/Play:** Press the space bar to pause playback.

### Display Options

When hovering over the timeline, you can:

- Scroll to scroll through the timeline.
- Shift + Scroll to make note spacing wider (increases scroll speed too).
- Ctrl + Scroll to increase row distance.

## Images

Some images of the editor in action.

![koigokoro](images\koigokoro.png)
Beatmap: [Icon for Hire - Koigokoro [Love] (Mualani, 10.30*)](https://rhythmtyper.net/beatmap/p5stw45z7wzp)

![make a move](images\makeamove.png)
[Icon For Hire - Make a Move (Speed Up Ver.) \[WE DONT KNOW WHATS GOING ON\] (Toiletwasher, 16.57*)](https://rhythmtyper.net/beatmap/1gkqcs2vslew)



