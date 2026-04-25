// The Sieve TTRPG Document
#set page(
  width: 5.5in,
  height: 8.5in,
  margin: (top: 0.5in, bottom: 0.5in, left: 0.4in, right: 0.4in),
  columns: 2,
)

#set text(
  font: ("Palatino", "Palatino Linotype", "Georgia", "serif"),
  size: 9pt,
  hyphenate: true,
)

#set par(
  justify: false,
  leading: 0.6em,
  first-line-indent: 0pt,
)

#set heading(numbering: none)

#show heading.where(level: 1): it => {
  set text(size: 16pt, weight: "bold")
  block(above: 0.8em, below: 0.5em)[#it.body]
}

#show heading.where(level: 2): it => {
  set text(size: 12pt, weight: "bold")
  block(above: 0.7em, below: 0.4em)[#it.body]
}

#show heading.where(level: 3): it => {
  set text(size: 10pt, weight: "bold")
  block(above: 0.5em, below: 0.3em)[#it.body]
}

// OSR-style stat block - simple shaded box
#let stat-block(name, content) = {
  block(
    width: 100%,
    fill: rgb("#e8e8e8"),
    inset: 8pt,
    radius: 2pt,
    breakable: false,
  )[
    #content
  ]
}

// Boxed text (read-aloud) styling
#let boxed-text(content) = {
  block(
    width: 100%,
    fill: rgb("#f4f4f0"),
    stroke: 0.5pt + rgb("#999"),
    inset: 8pt,
    radius: 0pt,
  )[
    #set text(style: "italic")
    #content
  ]
}

= The Sieve Style Guide

_A reference for writing TTRPG modules with The Sieve_

== Overview

The Sieve converts markdown documents into half-letter (5.5" x 8.5") PDFs optimized for booklet printing. This guide demonstrates all supported features.

== Basic Formatting

=== Text Styles

Regular paragraph text flows naturally in two columns. Keep paragraphs concise for better readability in the narrow column format.

Use *bold text* for important terms, monster names, or key items. Use _italic text_ for emphasis, book titles, or flavor text. You can combine _*bold and italic*_ when needed.

=== Headings

Use heading levels to organize your content:

- *Level 1* (`#`): Adventure or chapter titles
- *Level 2* (`##`): Major sections
- *Level 3* (`###`): Subsections
- *Level 4* (`####`): Minor divisions (NPC names, room numbers)

#pagebreak()

== Boxed Text

Use boxed text for read-aloud descriptions. Players expect this distinctive styling.

#boxed-text[The ancient door groans open, revealing a chamber thick with dust. Cobwebs drape the corners like funeral shrouds, and the air carries the faint scent of decay. In the center of the room, a stone pedestal holds a gleaming silver chalice.]

Keep boxed text concise—aim for 2-4 sentences. Long read-aloud sections lose player attention.

#boxed-text["You dare enter my domain?" The voice echoes from everywhere and nowhere. "Then you shall join the others who came before."]

== Stat Blocks

Stat blocks use a simple YAML-like format. The Sieve renders them in OSR style with a shaded background.

#stat-block["Goblin"][
*Goblin*, HD 1-1, AC 6 \[13\], MV 30'
*Atk* Spear: 1d6 damage.; Shortbow: 1d6 damage, range 50'.
_Infravision_: 60' range. _Cowardly_: Flees when outnumbered.
*Save* F0, *Morale* 7

]

For named NPCs or bosses, add more detail:

#stat-block["Grimshaw the Defiler"][
*Grimshaw the Defiler*, HD 45, AC 3 \[16\], MV 30'
*Atk* Corrupted Blade: 1d8+2 damage, target saves or gains 1 corruption.; Life Drain (1/day): Touch attack, 2d6 damage, Grimshaw heals equal amount.
_Undead_: Immune to sleep, charm, and poison. _Unholy Aura_: Living creatures within 10' take -1 to attacks. _Regeneration_: Heals 2 HP per round unless damaged by fire or holy water.
*Save* F8, *Morale* 500

]

#pagebreak()

== Multi-Column Layouts

Use columns for side-by-side content like room descriptions or comparison tables.

#grid(columns: 2, gutter: 0.8em,
  [=== The Western Passage

The passage slopes downward, its walls slick with moisture. Phosphorescent fungi provide dim light. The tunnel continues 60' before opening into Area 7.

\*\*Hazard:\*\* The floor is slippery. Characters moving faster than half speed must save or fall prone.
],
  [=== The Eastern Passage

Dry and dusty, this passage shows signs of recent traffic—footprints in the dust, a discarded torch stub. It leads 40' to Area 8.

\*\*Clue:\*\* A successful tracking check reveals goblin footprints heading east, and larger bootprints heading west.
],
)

Columns work well for:

- Parallel room descriptions
- NPC conversation options
- Faction comparisons
- Quick reference tables

== Tables

Use standard markdown tables for treasure, encounters, or reference data.

=== Random Encounters (1d6)

#table(
  columns: 3,
  inset: 6pt,
  align: (auto, auto, auto),
  table.header([*Roll*], [*Encounter*], [*Number*]),
  [1],
  [Giant Rats],
  [2d4],
  [2],
  [Goblin Patrol],
  [1d6+1],
  [3],
  [Wandering Merchant],
  [1],
  [4],
  [Skeleton Warriors],
  [1d4],
  [5],
  [Pit Trap],
  [—],
  [6],
  [No encounter],
  [—],
)

=== Treasure Table

#table(
  columns: 3,
  inset: 6pt,
  align: (auto, auto, auto),
  table.header([*Item*], [*Value*], [*Weight*]),
  [Silver chalice],
  [50 gp],
  [1 lb],
  [Ruby pendant],
  [200 gp],
  [—],
  [Spell scroll (Sleep)],
  [100 gp],
  [—],
  [Potion of Healing],
  [50 gp],
  [0.5 lb],
)

#pagebreak()

== Lists

=== Unordered Lists

Use bullet points for equipment, features, or options:

- Torch (1 hour light, 30' radius)
- Rope, 50' (hemp, holds 500 lbs)
- Iron spikes (12)
- Rations, dried (7 days)

=== Ordered Lists

Use numbered lists for sequences or ranked items:

1. The cultists perform their ritual at midnight
2. The portal opens, releasing 2d4 shadow demons
3. The high priest attempts to summon their dark god
4. If interrupted, the priest fights to the death

=== Nested Lists

- - Entry Hall (Area 1) - Guard Room (Area 2) - Kitchen (Area 3) *Ground Floor*
- - Master Bedroom (Area 4) - Study (Area 5) - Secret Room (Area 6) *Upper Floor*

== Page Breaks

Insert manual page breaks with an HTML comment:

```
<!-- pagebreak -->
```

Use page breaks before major sections, or to prevent stat blocks from splitting across pages.

#pagebreak()

== Special Characters

Some characters need care in markdown:

- Asterisks in game notation: HD 3\* means "has special ability"
- Armor class notation: AC 5 \[14\] (descending \[ascending\])
- Dice notation: 2d6+1, 1d8-1, 3d4
- Currency: 50 gp, 100 sp, 1,000 cp
- Measurements: 30', 60', 10' x 10'

== NPC Quick Reference Format

Here's a recommended format for NPCs:

==== Thornwick the Fence

_Human, middle-aged, nervous disposition_

*Wants:* To make money without drawing attention *Knows:* Location of the thieves' guild, black market prices *Secret:* He's actually an informant for the city watch

*Stats:* HD 1, AC 9 \[10\], Dagger 1d4

#line(length: 100%)

==== Sister Margaux

_Human, elderly, stern but kind_

*Wants:* To protect the village from evil *Knows:* Local history, herbal remedies, exorcism rites *Secret:* She was an adventurer in her youth and still has her magic sword hidden

*Stats:* Cleric 5, AC 7 \[12\], Mace 1d6, Spells: Cure Light Wounds x2, Hold Person

#pagebreak()

== Complete Example: Mini-Dungeon

=== The Goblin Warrens

_A lair for 4-6 characters of levels 1-2_

#boxed-text[The hillside cave mouth gapes like a wound in the earth. Crude totems of bone and feather flank the entrance. From within comes the flicker of firelight and the chittering of goblin speech.]

==== Area 1: Guard Post

Two goblins watch the entrance, playing dice. They attack intruders on sight but flee to warn others if clearly outmatched.

#stat-block["Goblin Guard"][
*Goblin Guard*, HD 4, AC 6 \[13\], MV 30'
*Atk* Spear: 1d6 damage.
_Infravision_: 60' range.
*Save* F0, *Morale* 7

]

*Treasure:* 2d6 sp between them, bone dice worth 5 cp.

==== Area 2: Common Room

#grid(columns: 2, gutter: 0.8em,
  [=== Western Alcove

Sleeping pallets for six goblins. A search reveals a hidden pouch with 15 gp under one mattress.
],
  [=== Eastern Alcove

Food storage—mostly rotting meat and stolen vegetables. A cask of surprisingly decent ale (worth 5 gp intact).
],
)

==== Area 3: Chief's Chamber

The goblin chief lounges on a throne of salvaged furniture, attended by his two bodyguards.

#stat-block["Goblin Chief"][
*Goblin Chief*, HD 12, AC 5 \[14\], MV 30'
*Atk* Morningstar: 1d6+1 damage.; Javelin: 1d4 damage, range 30'.
_Infravision_: 60' range. _Bully_: Nearby goblins gain +1 morale while chief lives.
*Save* F2, *Morale* 25

]

*Treasure:* Chief wears a gold chain (75 gp), carries a pouch with 30 gp, and keeps a locked chest (15 gp, 200 sp, potion of healing) under his throne.

#line(length: 100%)

_End of Style Guide_

