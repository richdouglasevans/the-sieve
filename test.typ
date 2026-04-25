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

= The Goblin's Lair

_A short adventure for 4-6 characters of levels 1-3_

== Introduction

Deep in the Thornwood Forest lies an ancient cave system, now home to a band of goblins led by their cunning boss, Snaggle. The goblins have been raiding nearby farms, and the local village has posted a bounty for anyone brave enough to clear them out.

#boxed-text[The path through the forest grows darker as ancient oaks crowd overhead. Ahead, you spot a rocky hillside with a yawning cave mouth. Bones litter the ground near the entrance, and the stench of unwashed goblin wafts from within.]

== Area 1: Cave Entrance

The entrance is ten feet wide and slopes gently downward. Two goblin sentries hide behind rocks just inside, watching for intruders.

#stat-block["Goblin"][
*Goblin*, HD 1-1, AC 6 [13], MV 40'
*Atk* Weapon: 1d6 damage.
_Infravision_: 60' range.
*Save* F0, *Morale* 7

]

#pagebreak()

== Area 2: The Main Chamber

The tunnel opens into a large cavern lit by a smoky fire pit in the center. Here, the goblin boss holds court with his minions.

#grid(columns: 2, gutter: 0.8em,
  [=== West Side

The western wall has several crude bedrolls and a pile of stolen goods: farming tools, a few coins, and some rotting food.
],
  [=== East Side

The eastern wall features a raised platform where the goblin boss sits on a makeshift throne of bones and leather.
],
)

== The Goblin Boss

Snaggle is cunning for a goblin. He wears a battered chain shirt taken from a fallen adventurer and wields a shortsword with surprising skill.

#stat-block["Snaggle, Goblin Boss"][
*Snaggle, Goblin Boss*, HD 3, AC 4 [15], MV 40'
*Atk* Shortsword: 1d6+1 damage.; Thrown Dagger: 1d4 damage, range 20'.
_Cunning_: Snaggle always acts first in combat unless surprised. _Cowardly_: If reduced to 1 HP, Snaggle will attempt to flee or surrender.
*Save* F2, *Morale* 8

]

== Treasure

#table(
  columns: 3,
  inset: 6pt,
  align: (auto, auto, auto),
  table.header([*Item*], [*Value*], [*Notes*]),
  [Coins],
  [35 gp],
  [Mixed copper, silver, gold],
  [Silver ring],
  [25 gp],
  [Engraved with initials "M.T."],
  [Healing potion],
  [50 gp],
  [Restores 1d6+1 HP],
  [Shortsword +1],
  [500 gp],
  [Snaggle's prized weapon],
)

#line(length: 100%)

== Conclusion

#boxed-text[With the goblins defeated, the cave falls silent except for the crackling of the dying fire. The villagers will be relieved to hear of your success, and the farms can rest easy once more.]

_The party earns 150 XP each for completing this adventure._

