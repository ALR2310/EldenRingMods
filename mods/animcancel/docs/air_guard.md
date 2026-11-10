# Guarding in the air

Status: **Rejected.** Cannot be done with a DLL alone. Conventions:
[research notes](../../../docs/research/README.md).

## Question

Can guard be made to work while jumping, like air blocking in other games?

## Result

The game has no blocking state in the air. Forcing the guard action in mid-air makes the game pull
the character to the ground and reset its state every frame, which is unstable. Doing it properly needs
a new animation and changes to the behavior files (HKS/TAE), beyond what a DLL can do.

## What was tried

- A `Jump` animation group where only the guard bit is opened while airborne, and the roll and weapon
  swap bits are not.
- A probe that logged the behavior events emitted while holding L1 in the air, together with
  `standing_on_solid_ground`.

Neither is kept in the repository. Do not retry unless behavior files can be changed outside the DLL.
