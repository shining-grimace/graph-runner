>Player movements, behaviours of other entities, and so on which all must be polished individually and all be brought to the player's attention during the game. Consider possibilities of where orthogonal mechanics intersect to create interesting challenges.
# Player
## States

| State          | Info                                                                                                                 | Attachment Component | Special Move Component |
| -------------- | -------------------------------------------------------------------------------------------------------------------- | -------------------- | ---------------------- |
| Standing       |                                                                                                                      | Grounded             | None                   |
| Landing        | Airborne onto ground if above speed threshold and no direction input or opposes motion                               | Grounded             | Landing                |
| Rolling        | Airborne onto ground if above speed threshold and direction input supports motion, or grounded and pressing a button | Grounded / None      | Rolling                |
| Walking        | Acceleration / deceleration                                                                                          | Grounded             | None                   |
| Running        | Acceleration / deceleration                                                                                          | Grounded             | Running                |
| Halting        |                                                                                                                      | Grounded             | Halting                |
| Sliding        | Slope was too steep                                                                                                  | Grounded             | Sliding                |
| Jumping        |                                                                                                                      | None                 | Jumping                |
| Falling        |                                                                                                                      | None                 | None                   |
| Diving         |                                                                                                                      | None                 | Diving                 |
| Wall sticking  | Stationary on a wall                                                                                                 | Walled               | None                   |
| Wall slipping  | Sliding down a wall                                                                                                  | Walled               | Sliding                |
| Entering water |                                                                                                                      | Immersed             | Halting                |
| Swimming       |                                                                                                                      | Immersed             | None                   |
| Streaming      |                                                                                                                      | Immersed             | Diving                 |
| Floating       |                                                                                                                      | Floating             | None                   |
## Transitions

| v From \| To > | Stand                           | Land                                 | Roll                               | Walk                       | Run                         | Halt                          | Slide              | Jump  | Fall                           | Dive       | Wall stick | Wall slide | Enter water   | Swim  | Stream        | Float           |
| -------------- | ------------------------------- | ------------------------------------ | ---------------------------------- | -------------------------- | --------------------------- | ----------------------------- | ------------------ | ----- | ------------------------------ | ---------- | ---------- | ---------- | ------------- | ----- | ------------- | --------------- |
| Stand          |                                 | X                                    | X                                  | Dir input                  | X                           | X                             | X                  | Input | Loss of grounding              | X          | X          | X          | X             | X     | X             | X               |
| Land           | Delay                           |                                      | Dir input, input                   | Dir input, delay           | X                           | X                             | X                  | X     | Loss of grounding              | X          | X          | X          | X             | X     | X             | X               |
| Roll           | No dir input, delay             | X                                    |                                    | X                          | Dir input, delay            | X                             | Slope became steep | X     | Loss of grounding, delay       | X          | X          | X          | X             | X     | Entered water | X               |
| Walk           | Decelerated under no dir input  | X                                    | X                                  |                            | Dir input                   | X                             | Slope became steep | Input | Loss of grounding              | X          | X          | X          | Entered water | X     | X             | X               |
| Run            | X                               | X                                    | Dir input                          | Deceleration               |                             | Dir input, sharp deceleration | Slope became steep | Input | Loss of grounding              | X          | X          | X          | Entered water | X     | X             | X               |
| Halt           | No dir input, delay             | X                                    | Stationary, cool new move on jump? | Dir input, delay           | X                           |                               | X                  | X     | Loss of grounding              | Input      | X          | X          | X             | X     | X             | X               |
| Slide          | Decelerated on non-steep ground | X                                    | X                                  | X                          | X                           | X                             |                    | Input | Loss of grounding              | X          | X          | X          | Entered water | X     | X             | X               |
| Jump           | Became grounded                 | X                                    | X                                  | X                          | X                           | X                             | X                  |       | Vertical speed became negative | X          | Hit a wall | X          | Entered water | X     | X             | X               |
| Fall           | Became grounded                 | Became grounded, high vertical speed | X                                  | Became grounded. low speed | Became grounded, high speed | X                             | X                  | X     |                                | Input      | Hit a wall | X          | Entered water | X     | X             | X               |
| Dive           | X                               | X                                    | X                                  | X                          | X                           | X                             | Became grounded    | X     | Hit a wall                     |            | X          | X          | X             | X     | Entered water | X               |
| Wall stick     | X                               | X                                    | Input (breakable wall)             | X                          | X                           | X                             | X                  | Input | X                              | Input      |            | Delay      | X             | X     | X             | X               |
| Wall slide     | Became grounded                 | X                                    | X                                  | X                          | X                           | X                             | X                  | Input | Loss of wall                   | Input      | X          |            | Entered water | X     | X             | X               |
| Enter water    | X                               | X                                    | X                                  | X                          | X                           | X                             | X                  | X     | Left water                     | X          | X          | X          |               | Delay | Input         | Rose to surface |
| Swim           | X                               | X                                    | X                                  | X                          | X                           | X                             | X                  | X     | Left water                     | X          | X          | X          | X             |       | Input         | Rose to surface |
| Stream         | X                               | X                                    | X                                  | X                          | X                           | X                             | X                  | X     | X                              | Left water | X          | X          | X             | Delay |               | X               |
| Float          | X                               | X                                    | X                                  | X                          | X                           | X                             | X                  | Input | Left water                     | X          | X          | X          | X             | Input | Input         |                 |
## Controls
Controls are context-sensitive; what the primary action input does will be displayed in the HUD just like N64 Zelda.

Input types:
- Button inputs (discrete and continuous direction, primary button, secondary button)
- Gamepad inputs (continuous direction, primary button, secondary button)
- Touchscreen inputs
Control parameters:
- App state (main menu, pause menu, gameplay)
- Player gameplay state (attachment, special move, current velocity)
# World Mechanics
## List

| Item           | Notes                                |
| -------------- | ------------------------------------ |
| Platforming    | General movement on land             |
| Wall jumping   | Jump higher before sliding           |
| Swimming       |                                      |
| Sliding        |                                      |
| Swamp          | Move slower in swamp                 |
| Breaking trees | Attach to tree and burst through     |
| Thieving       | Activist chasing to take cargo       |
| Deflecting     | Badgers knock back                   |
| Tide           | Water rising and falling             |
| Conversing     | Elect to talk to an NPC, game pauses |

## Orthogonal Crossovers

|                | Platforming                 | Wall jumping | Swimming | Sliding | Swamp | Breaking trees | Thieving | Deflecting | Tide | Conversing |
| -------------- | --------------------------- | ------------ | -------- | ------- | ----- | -------------- | -------- | ---------- | ---- | ---------- |
| Platforming    | -                           | -            | -        | -       | -     | -              | -        | -          | -    | -          |
| Wall jumping   |                             | -            | -        | -       | -     | -              | -        | -          | -    | -          |
| Swimming       |                             |              | -        | -       | -     | -              | -        | -          | -    | -          |
| Sliding        |                             |              |          | -       | -     | -              | -        | -          | -    | -          |
| Swamp          |                             |              |          |         | -     | -              | -        | -          | -    | -          |
| Breaking trees |                             |              |          |         |       | -              | -        | -          | -    | -          |
| Thieving       |                             |              |          |         |       |                | -        | -          | -    | -          |
| Deflecting     | Danger of knocking into pit |              |          |         |       |                |          | -          | -    | -          |
| Tide           |                             |              |          |         |       |                |          |            | -    | -          |
| Conversing     |                             |              |          |         |       |                |          |            |      | -          |
# Implementations
## Game States
- Loading
- Splash screen
- Main menu
- Pause menu
- Interactive dialog
- Gameplay
## Pacing
Important mechanics per beat:
1:
- Conversing introduced
- Platforming introduced
- Swamp introduced
- Sliding introduced
2:
- Conversing
3:
- Platforming
- Wall jumping introduced
- Breaking trees introduced
4:
- Conversing
5:
- Platforming
- Swimming introduced
- Tide introduced
6:
- Conversing
7:
- Platforming
- Thieving introduced
- Breaking trees
- Swimming
- Swamp
- Sliding
8:
- Conversing
9:
- Platforming
- Deflecting introduced
- Wall jumping
10:
- Conversing
11:
- Platforming
12:
- Conversing
13:
- Platforming
14:

## Camera
- Player holds a `CameraTracking` component
- An invisible entity also holds that component, and hovers in front of the player (but locks onto the dialog entity during interactive dialog)
- Focus entities (talking NPCs, etc) given that component as needed
- Move to keep those entities in view within a bounding box
- Momentum on both XY movement and zoom
## Dialog
- Dialog is always in the background or is optional
- The entity that the dialog relates to will gain a `CameraTracking` component
- Background dialog is a temporary hovering box above the entity
- Interactive dialog is a box presented in the HUD which can flick pages when the user presses a button
