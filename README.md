# Small experiment
This project is merely a small experiment to see how a factory-like game could be implemented. Takes inspiration from the following games:
- Satisfactory
- Factorio
- Minecraft Tech Mods (GregTech, Mekanism, Thermal Series, Create)

# Systems
## Resources
A resource is a "thing" with a count and name:
- 10 iron ore
- 2 copper ingots

Examples of resources include raw products, processed products, intermediates, and other stuff.

## Machines
Machines are simple black boxes that take some input resources (items, fluids, etc), process them, and output some other resources.
Machines have recipes associated with them. For example:
- The `process raw iron` recipe takes `x raw iron` and processes it to `y iron ingots`

Some recipes should be locked behind specific types of machines. A smelter should only be able to smelt resources / crushed ores. It should not be able to do anything other than that.

Machines have hatches, which are the I/O ports.

## Belts
Belts transfer items from one hatch to another


# Ideas
## Multiblock Like Progression
### Building
To create machines, instead of crafting them in a table (like in Minecraft & GregTech) or placing it down immediately like in Satisfactory, you need to craft the individual components and place them in specific spots in the world.
For example, to create a "furnace" you could do the following:
- Create the interior of the kiln
- Build the heating mantle / components
- Build the door & latching mechanism & whatever
- Build the exterior

### Stat Modifications
This can be extended to other machines where each individual component (either structural, mechanical, electrical) serves a specific purpose and *does* affect how the machine functions. This should incentivize the player *and* give the player the ability to scale vertically by upgrading machines instead of adding more machines. Plus, each of these component archetypes could have a specific stat they are tied to, and upgrading one increases / decreases that stat, depending on which means "better". Example:
- efficiency. "how much power this machine consumes at idle / full power"
- speed.
- (specific to furnaces machines) max temperature. certain recipes could be locked behind a minimum temperature, and upgrading certain parts of the furnace might help it reach higher temperatures

### Auto-Building
A problem with this mechanic is that building machines by hand would become tedious. Maybe that could incentivize the player to build autocrafters *in world space* (using gantries / mechanical arms / other stuff) that builds the machines and stores them somewhere (warehouse / dedicated world space storage for machines). Of course, if we stick with such a system, then we cannot allow the player to simply store the machines in their inventory. If crafting is going to be in world-space then storage must also be in world space (I feel like there should be exceptions to this rule. Should apply to large / huge items; small items should still be craftable by hand)

### Repairability / Maintenance
Another bonus that we get from this sort of system is replaceable parts. If a certain component gets damaged / wears out, you can replace it. This adds a layer of "maintenance" that you have to uphold to keep your factory running smoothly. 
- Needs to be balanced so that it doesn't feel like a chore / annoying
- Should have repercussions if you neglect repairing your machines for too long
    - Could be a loss in efficiency or dysfunction

### Summary
- Build machines in world space
- Machines not stored in inventory. Must be moved / placed AOT
- Machines can be manufactured automatically and stored in world space
- Machines are less of a "black box"; you have a bit more insight into what happens inside of it; there are multiple components:
    - Components are replaceable and upgradeable
    - Components wear down. Neglect yields inefficiencies. Maintenance required


## Macro Level Puzzle
