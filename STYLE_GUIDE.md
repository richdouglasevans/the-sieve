# The Sieve Style Guide

*A reference for writing TTRPG modules with The Sieve*

## Overview

The Sieve converts markdown documents into half-letter (5.5" x 8.5") PDFs optimized for booklet printing. This guide demonstrates all supported features.

## Basic Formatting

### Text Styles

Regular paragraph text flows naturally in two columns. Keep paragraphs concise for better readability in the narrow column format.

Use **bold text** for important terms, monster names, or key items. Use *italic text* for emphasis, book titles, or flavor text. You can combine ***bold and italic*** when needed.

### Headings

Use heading levels to organize your content:

- **Level 1** (`#`): Adventure or chapter titles
- **Level 2** (`##`): Major sections
- **Level 3** (`###`): Subsections
- **Level 4** (`####`): Minor divisions (NPC names, room numbers)

<!-- pagebreak -->

## Boxed Text

Use boxed text for read-aloud descriptions. Players expect this distinctive styling.

```boxed
The ancient door groans open, revealing a chamber thick with dust. Cobwebs drape the corners like funeral shrouds, and the air carries the faint scent of decay. In the center of the room, a stone pedestal holds a gleaming silver chalice.
```

Keep boxed text concise—aim for 2-4 sentences. Long read-aloud sections lose player attention.

```boxed
"You dare enter my domain?" The voice echoes from everywhere and nowhere. "Then you shall join the others who came before."
```

## Stat Blocks

Stat blocks render any text with a shaded background. Format the content however you like—The Sieve just provides the box.

///
Goblin — HD 1-1, AC 6 [13], MV 30'
Atk: Spear 1d6 or Shortbow 1d6
Special: Infravision 60'
Save: F0, Morale: 7
///

For more detailed stat blocks:

///
Grimshaw the Defiler — HD 8 (45 HP), AC 3 [16], MV 30'
Atk: Corrupted Blade 1d8+2 (save or gain corruption)
Special: Undead (immune to sleep, charm, poison), Unholy Aura (-1 to attacks within 10'), Regeneration (2 HP/round, stopped by fire or holy water)
Save: F8, Morale: 12
///

<!-- pagebreak -->

## Tables

Use standard markdown tables for treasure, encounters, or reference data.

### Random Encounters (1d6)

| Roll | Encounter | Number |
|------|-----------|--------|
| 1 | Giant Rats | 2d4 |
| 2 | Goblin Patrol | 1d6+1 |
| 3 | Wandering Merchant | 1 |
| 4 | Skeleton Warriors | 1d4 |
| 5 | Pit Trap | — |
| 6 | No encounter | — |

### Treasure Table

| Item | Value | Weight |
|------|-------|--------|
| Silver chalice | 50 gp | 1 lb |
| Ruby pendant | 200 gp | — |
| Spell scroll (Sleep) | 100 gp | — |
| Potion of Healing | 50 gp | 0.5 lb |

<!-- pagebreak -->

## Lists

### Unordered Lists

Use bullet points for equipment, features, or options:

- Torch (1 hour light, 30' radius)
- Rope, 50' (hemp, holds 500 lbs)
- Iron spikes (12)
- Rations, dried (7 days)

### Ordered Lists

Use numbered lists for sequences or ranked items:

1. The cultists perform their ritual at midnight
2. The portal opens, releasing 2d4 shadow demons
3. The high priest attempts to summon their dark god
4. If interrupted, the priest fights to the death

### Nested Lists

- **Ground Floor**
  - Entry Hall (Area 1)
  - Guard Room (Area 2)
  - Kitchen (Area 3)
- **Upper Floor**
  - Master Bedroom (Area 4)
  - Study (Area 5)
  - Secret Room (Area 6)

## Page Breaks

Insert manual page breaks with an HTML comment:

```
<!-- pagebreak -->
```

Use page breaks before major sections, or to prevent stat blocks from splitting across pages.

<!-- pagebreak -->

## Column Layout

The default layout is two columns, optimized for half-letter booklets. You can switch to single-column for title pages, full-width content, or large illustrations.

### Switching to Single Column

<!-- 1-column -->

Use `<!-- 1-column -->` to switch to single-column layout. This is useful for:

- **Title pages** with large artwork
- **Full-width maps** that need more space
- **Large tables** that don't fit in narrow columns
- **Important notices** that should stand out

This paragraph spans the full page width, making it easier to read longer content or display wide images.

### Switching Back to Two Columns

Use `<!-- 2-column -->` to return to the standard two-column layout:

<!-- 2-column -->

Now we're back to two columns. The text flows normally again, optimized for the half-letter format. Most adventure content works best in two columns.

## Special Characters

Some characters need care in markdown:

- Asterisks in game notation: HD 3* means "has special ability"
- Armor class notation: AC 5 [14] (descending [ascending])
- Dice notation: 2d6+1, 1d8-1, 3d4
- Currency: 50 gp, 100 sp, 1,000 cp
- Measurements: 30', 60', 10' x 10'

## NPC Quick Reference Format

Here's a recommended format for NPCs:

#### Thornwick the Fence

*Human, middle-aged, nervous disposition*

**Wants:** To make money without drawing attention
**Knows:** Location of the thieves' guild, black market prices
**Secret:** He's actually an informant for the city watch

**Stats:** HD 1, AC 9 [10], Dagger 1d4

---

#### Sister Margaux

*Human, elderly, stern but kind*

**Wants:** To protect the village from evil
**Knows:** Local history, herbal remedies, exorcism rites
**Secret:** She was an adventurer in her youth and still has her magic sword hidden

**Stats:** Cleric 5, AC 7 [12], Mace 1d6, Spells: Cure Light Wounds x2, Hold Person

<!-- pagebreak -->

## Complete Example: Mini-Dungeon

### The Goblin Warrens

*A lair for 4-6 characters of levels 1-2*

```boxed
The hillside cave mouth gapes like a wound in the earth. Crude totems of bone and feather flank the entrance. From within comes the flicker of firelight and the chittering of goblin speech.
```

#### Area 1: Guard Post

Two goblins watch the entrance, playing dice. They attack intruders on sight but flee to warn others if clearly outmatched.

///
Goblin Guard — HD 1-1 (4 HP), AC 6 [13], MV 30'
Atk: Spear 1d6
Special: Infravision 60'
Save: F0, Morale: 7
///

**Treasure:** 2d6 sp between them, bone dice worth 5 cp.

#### Area 2: Common Room

```columns
### Western Alcove

Sleeping pallets for six goblins. A search reveals a hidden pouch with 15 gp under one mattress.

---

### Eastern Alcove

Food storage—mostly rotting meat and stolen vegetables. A cask of surprisingly decent ale (worth 5 gp intact).
```

#### Area 3: Chief's Chamber

The goblin chief lounges on a throne of salvaged furniture, attended by his two bodyguards.

///
Goblin Chief — HD 2 (12 HP), AC 5 [14], MV 30'
Atk: Morningstar 1d6+1 or Javelin 1d4 (30')
Special: Infravision 60', Bully (nearby goblins +1 morale)
Save: F2, Morale: 8
///

**Treasure:** Chief wears a gold chain (75 gp), carries a pouch with 30 gp, and keeps a locked chest (15 gp, 200 sp, potion of healing) under his throne.

---

*End of Style Guide*
