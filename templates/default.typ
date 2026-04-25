// The Sieve Default Template
// Half-letter (5.5" x 8.5") format for TTRPG booklets

#let page-setup = (
  paper: "us-letter",
  width: 5.5in,
  height: 8.5in,
  margin: (top: 0.6in, bottom: 0.6in, left: 0.5in, right: 0.5in),
)

#let body-font = "Linux Libertine"
#let heading-font = "Linux Libertine"

#let primary-color = rgb("#922610")
#let secondary-color = rgb("#8b7355")
#let stat-block-bg = rgb("#f5e6d3")
#let boxed-text-bg = rgb("#e8e4dc")

// Stat block component
#let stat-block(name, content) = {
  block(
    width: 100%,
    fill: stat-block-bg,
    stroke: (
      top: 3pt + primary-color,
      bottom: 3pt + primary-color,
    ),
    inset: 12pt,
    breakable: false,
  )[
    #set text(size: 9pt)
    #content
  ]
}

// Boxed text (read-aloud) component
#let boxed-text(content) = {
  block(
    width: 100%,
    fill: boxed-text-bg,
    stroke: 1pt + secondary-color,
    inset: 10pt,
    radius: 2pt,
  )[
    #set text(style: "italic", size: 10pt)
    #content
  ]
}

// Ability score row component
#let ability-scores(str: none, dex: none, con: none, int: none, wis: none, cha: none) = {
  let format-score(score) = {
    if score == none { return [--] }
    let modifier = calc.floor((score - 10) / 2)
    let sign = if modifier >= 0 { "+" } else { "" }
    [#score (#sign#modifier)]
  }

  grid(
    columns: 6,
    gutter: 4pt,
    align(center)[*STR*], align(center)[*DEX*], align(center)[*CON*],
    align(center)[*INT*], align(center)[*WIS*], align(center)[*CHA*],
    align(center)[#format-score(str)],
    align(center)[#format-score(dex)],
    align(center)[#format-score(con)],
    align(center)[#format-score(int)],
    align(center)[#format-score(wis)],
    align(center)[#format-score(cha)],
  )
}

// Section divider
#let stat-divider = line(length: 100%, stroke: 1pt + primary-color)

// Trait/Action entry
#let entry(name, description) = {
  [*_#name._* #description]
}

// Section header (for Actions, Reactions, etc.)
#let stat-section(title) = {
  text(size: 12pt, weight: "bold", fill: primary-color)[#title]
  stat-divider
}
