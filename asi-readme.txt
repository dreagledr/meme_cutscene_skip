meme_cutscene_skip
In-engine cutscene skip for Metal Gear Rising: Revengeance (PC, Steam).
https://github.com/dreagledr/meme_cutscene_skip

WHAT IS IN THIS ARCHIVE
  plugins\meme_cutscene_skip_lib.asi - the mod itself. That is all the game
  loads; this readme.txt is only for you and can be deleted.

INSTALL
  1. You need an ASI loader: the game does not load .asi plugins by itself.
     If you don't have one yet, download the latest Win32 d3d9.dll from
     https://github.com/ThirteenAG/Ultimate-ASI-Loader/releases
  2. Copy d3d9.dll and the plugins folder into the game's root folder, so that
     the tree looks like this:

       Metal Gear Rising REVENGEANCE\
       |-- METAL GEAR RISING REVENGEANCE.EXE
       |-- d3d9.dll
       |-- plugins\
            |-- meme_cutscene_skip_lib.asi

  3. Start the game as usual. The mod loads with it - no launcher, no
     injection. It skips the cutscenes "like on consoles" in the P370_RESTART
     and P370_IN scenes.

UNINSTALL
  Delete plugins\meme_cutscene_skip_lib.asi, and d3d9.dll unless another mod
  needs that loader. That is all - the mod keeps no files of its own.

ALTERNATIVE
  The same release also ships meme_cutscene_skip_injector.zip - a launcher that
  starts the game and injects the very same mod, so no loader is needed.
