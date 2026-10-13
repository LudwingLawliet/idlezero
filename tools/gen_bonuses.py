"""Genera data/civ_bonuses.toml: cada bono se ancla a una línea del texto
oficial en inglés y cita la línea española equivalente de data/civs.json."""
import json, re, sys
root = sys.argv[1]
d = json.load(open(f'{root}/data/civs.json'))

def bullets(txt):
    paras = re.split(r'\n\s*\n', txt)
    parts = [re.sub(r'\s*\n\s*', ' ', p).strip() for p in paras[1].split('•')[1:]]
    team = re.sub(r'\s*\n\s*', ' ', paras[-1].split('\n', 1)[1]).strip() if len(paras) > 2 else None
    return [p.strip() for p in parts], team

ECO = "[202, 203, 221, 55, 182, 278, 279, 14, 13, 12, 213, 249]"  # mejoras económicas
AGES = "[101, 102, 103]"
MIL = "[12, 87, 101, 49]"  # cuarteles, galería, establo, taller de asedio
A = "approximate = true"

# civ -> [(subcadena de la línea inglesa o "TEAM", cuerpo TOML)]
S = {
 "Aztecs": [("Start with +50 gold", 'kind = "start_resources_add"\nresources = { gold = 50 }'),
            ("Villagers carry +3", 'kind = "carry_add"\ntasks = ["*"]\nvalue = 3'),
            ("Military Units train", f'kind = "production_speed_mult"\nbuildings = {MIL}\nby_age = {{ Dark = 1.15 }}')],
 "Bengalis": [("spawn 2 Villagers", 'kind = "age_villagers"\nvalue = 2')],
 "Berbers": [("Villagers move", 'kind = "villager_speed_mult"\nby_age = { Dark = 1.05, Feudal = 1.10 }')],
 "Bohemians": [("Mining Camp technologies free", 'kind = "tech_free"\ntechs = [55, 182, 278, 279]'),
               ("Blacksmiths and Universities", 'kind = "building_cost_add"\nbuildings = [103, 209]\nresources = { wood = -100 }')],
 "Britons": [("Shepherds", 'kind = "rate_mult"\ntasks = ["shepherd"]\nvalue = 1.25'),
             ("Town Centers cost -50% wood", 'kind = "building_cost_mult"\nbuildings = [109]\nresources = ["wood"]\nvalue = 0.5\nfrom_age = "Castle"'),
             ("TEAM", 'kind = "production_speed_mult"\nbuildings = [87]\nby_age = { Dark = 1.10 }')],
 "Bulgarians": [("Town Centers cost -50% stone", 'kind = "building_cost_mult"\nbuildings = [109]\nresources = ["stone"]\nvalue = 0.5')],
 "Burgundians": [("Economic upgrades", f'kind = "tech_cost_mult"\ntechs = {ECO}\nresources = ["food"]\nby_age = {{ Dark = 0.6 }}\n{A}  # "una edad antes" no se simula')],
 "Burmese": [("Lumber Camp technologies free", 'kind = "tech_free"\ntechs = [202, 203, 221]')],
 "Byzantines": [("Advancing to Imperial", 'kind = "tech_cost_mult"\ntechs = [103]\nby_age = { Dark = 0.67 }')],
 "Celts": [("Lumberjacks", 'kind = "rate_mult"\ntasks = ["lumberjack"]\nvalue = 1.15')],
 "Chinese": [("Start with +3 Villagers", 'kind = "start_villagers_add"\nvalue = 3'),
             ("Start with +3 Villagers", 'kind = "start_resources_add"\nresources = { wood = -50, food = -200 }'),
             ("Technologies cost", f'kind = "tech_cost_mult"\nby_age = {{ Feudal = 0.95, Castle = 0.90, Imperial = 0.85 }}\n{A}  # sin contar los avances de edad'),
             ("Town Centers +7", 'kind = "population_add"\nbuilding = 109\nvalue = 15'),
             ("TEAM", 'kind = "farm_food_mult"\nvalue = 1.10')],
 "Cumans": [("Archery Ranges and Stables", 'kind = "building_cost_add"\nbuildings = [87, 101]\nresources = { wood = -75 }')],
 "Danes": [("drop off +5% food", f'kind = "rate_mult"\ntasks = ["shepherd", "hunter", "forager", "farmer", "fisherman"]\nvalue = 1.05\n{A}  # entregar +5 % ≈ recolectar +5 %')],
 "Dravidians": [("Fishermen and Fishing Ships carry", 'kind = "carry_add"\ntasks = ["fisherman"]\nvalue = 15'),
                ("+200 wood", 'kind = "age_reward"\nresources = { wood = 200 }')],
 "Ethiopians": [("+100 gold and +100 food", 'kind = "age_reward"\nresources = { food = 100, gold = 100 }')],
 "Franks": [("Foragers", 'kind = "rate_mult"\ntasks = ["forager"]\nvalue = 1.15'),
            ("Mill technologies free", 'kind = "tech_free"\ntechs = [14, 13, 12]')],
 "Goths": [("Loom is researched instantly", 'kind = "research_speed_mult"\ntechs = [22]\nvalue = 1000.0'),
           ("Hunters carry +15", 'kind = "carry_add"\ntasks = ["hunter"]\nvalue = 15'),
           ("Hunters carry +15", 'kind = "source_amount_mult"\npools = ["boar", "deer"]\nvalue = 1.2'),
           ("TEAM", 'kind = "production_speed_mult"\nbuildings = [12]\nby_age = { Dark = 1.20 }')],
 "Hindustanis": [("Villagers cost", 'kind = "unit_cost_mult"\nunits = [83]\nby_age = { Dark = 0.92, Feudal = 0.87, Castle = 0.82, Imperial = 0.77 }')],
 "Huns": [("Do not need houses", 'kind = "no_houses"'),
          ("Do not need houses", 'kind = "start_resources_add"\nresources = { wood = -100 }'),
          ("TEAM", 'kind = "production_speed_mult"\nbuildings = [101]\nby_age = { Dark = 1.20 }')],
 "Incas": [("Houses and Settlements", 'kind = "population_add"\nbuilding = 70\nvalue = 5'),
           ("Buildings cost -15% stone", 'kind = "building_cost_mult"\nresources = ["stone"]\nvalue = 0.85')],
 "Italians": [("Advancing to the next Age costs -15%", f'kind = "tech_cost_mult"\ntechs = {AGES}\nby_age = {{ Dark = 0.85 }}')],
 "Japanese": [("Mills, Lumber- and Mining Camps cost -50%", 'kind = "building_cost_mult"\nbuildings = [68, 562, 584]\nvalue = 0.5')],
 "Khmer": [("No buildings required", 'kind = "no_age_buildings"')],
 "Koreans": [("Stone miners", 'kind = "rate_mult"\ntasks = ["stone_miner"]\nvalue = 1.20')],
 "Lithuanians": [("Each Town Center provides +100 food", 'kind = "start_resources_add"\nresources = { food = 100 }'),
                 ("Each Town Center provides +100 food", 'kind = "building_reward"\nbuildings = [109]\nresources = { food = 100 }')],
 "Malay": [("Advancing to the next Age is +66% faster", f'kind = "research_speed_mult"\ntechs = {AGES}\nvalue = 1.66')],
 "Malians": [("Buildings cost -15% wood", 'kind = "building_cost_mult"\nresources = ["wood"]\nvalue = 0.85'),
             ("drop off +10% more gold", f'kind = "rate_mult"\ntasks = ["gold_miner"]\nvalue = 1.10\n{A}  # entregar +10 % ≈ recolectar +10 %')],
 "Mapuche": [("Foragers drop off +25% food", f'kind = "rate_mult"\ntasks = ["forager"]\nvalue = 1.25\n{A}  # entregar +25 % ≈ recolectar +25 %')],
 "Mayans": [("Start with +1 Villager", 'kind = "start_villagers_add"\nvalue = 1'),
            ("Start with +1 Villager", 'kind = "start_resources_add"\nresources = { food = -50 }'),
            ("Resources last +15% longer", 'kind = "source_amount_mult"\nvalue = 1.15'),
            ("Foot Archers cost", 'kind = "unit_cost_mult"\nunits = [4, 24, 492]\nby_age = { Feudal = 0.9, Castle = 0.8, Imperial = 0.7 }')],
 "Mongols": [("Hunters work +40% faster", 'kind = "rate_mult"\ntasks = ["hunter"]\nvalue = 1.40')],
 "Muisca": [("Advancing to the next Age costs -50% gold", f'kind = "tech_cost_mult"\ntechs = {AGES}\nresources = ["gold"]\nby_age = {{ Dark = 0.5 }}'),
            ("TEAM", 'kind = "source_amount_mult"\npools = ["gold"]\nvalue = 1.15')],
 "Persians": [("Start with +50 wood and +50 food", 'kind = "start_resources_add"\nresources = { wood = 50, food = 50 }'),
              ("Town Centers and Docks", 'kind = "production_speed_mult"\nbuildings = [109, 45]\nby_age = { Dark = 1.05, Feudal = 1.10, Castle = 1.15, Imperial = 1.20 }')],
 "Portuguese": [("All units cost -20% gold", 'kind = "unit_cost_mult"\nresources = ["gold"]\nby_age = { Dark = 0.8 }'),
                ("TEAM", 'kind = "research_speed_mult"\nvalue = 1.25')],
 "Romans": [("Villagers gather, build", 'kind = "rate_mult"\ntasks = ["*"]\nvalue = 1.05'),
            ("Villagers gather, build", 'kind = "build_speed_mult"\nvalue = 1.05')],
 "Saracens": [("Markets cost -100 wood", 'kind = "building_cost_add"\nbuildings = [84]\nresources = { wood = -100 }')],
 "Saxons": [("Mills, Lumber- and Mining Camps provide", 'kind = "building_reward"\nbuildings = [68, 562, 584]\nresources = { food = 35, stone = 10 }')],
 "Sicilians": [("Start with +100 stone", 'kind = "start_resources_add"\nresources = { stone = 100 }'),
               ("Farm upgrades provide", 'kind = "farm_tech_mult"\nvalue = 2.25'),
               ("Town Centers built +100% faster", 'kind = "build_speed_mult"\nbuildings = [109]\nvalue = 2.0')],
 "Slavs": [("Farmers work +15% faster", 'kind = "rate_mult"\ntasks = ["farmer"]\nvalue = 1.15')],
 "Spanish": [("Builders work +30% faster", 'kind = "build_speed_mult"\nvalue = 1.30'),
             ("+20 gold for each technology", 'kind = "research_reward"\nresources = { gold = 20 }')],
 "Tatars": [("Livestock animals last +50% longer", 'kind = "source_amount_mult"\npools = ["sheep"]\nvalue = 1.5')],
 "Teutons": [("Farms cost -40%", 'kind = "building_cost_mult"\nbuildings = [50]\nvalue = 0.6')],
 "Tupi": [("Start with +25 of each resource", 'kind = "start_resources_add"\nresources = { food = 25, wood = 25, gold = 25, stone = 25 }')],
 "Turks": [("Gold miners work +25% faster", 'kind = "rate_mult"\ntasks = ["gold_miner"]\nvalue = 1.25')],
 "Vietnamese": [("Economic upgrades cost no wood", f'kind = "tech_cost_mult"\ntechs = {ECO}\nresources = ["wood"]\nby_age = {{ Dark = 0.0 }}\n{A}  # lista de mejoras económicas aproximada'),
                ("Economic upgrades cost no wood", f'kind = "research_speed_mult"\ntechs = {ECO}\nvalue = 2.0\n{A}')],
 "Vikings": [("Wheelbarrow, Hand Cart free", 'kind = "tech_free"\ntechs = [213, 249]')],
 "Wei": [("free Villager for each economic upgrade", f'kind = "tech_villagers"\ntechs = {ECO}\nvalue = 1\n{A}  # lista de mejoras económicas aproximada')],
 "Wu": [("Military production buildings and Docks provide +55 food", 'kind = "building_reward"\nbuildings = [12, 87, 101, 49, 45]\nresources = { food = 55 }'),
        ("TEAM", 'kind = "build_speed_mult"\nbuildings = [70]\nvalue = 2.0')],
}

# Descuentos y mejoras gratis de unidades y tecnologías (cuentan en objetivos
# con unidades o mejoras).
MORE = {
 "Berbers": [("Stable Units cost", 'kind = "unit_cost_mult"\ntrained_at = [101]\nby_age = { Castle = 0.85, Imperial = 0.80 }')],
 "Bulgarians": [("Militia-line upgrades free", 'kind = "tech_free"\ntechs = [222, 207, 217, 264]'),
                ("Blacksmith and Siege Workshop technologies", 'kind = "tech_cost_mult"\nresearched_at = [103, 49]\nresources = ["food"]\nby_age = { Dark = 0.5 }')],
 "Burgundians": [("Stable technologies cost -50%", 'kind = "tech_cost_mult"\nresearched_at = [101]\nby_age = { Dark = 0.5 }')],
 "Burmese": [("Monastery technologies cost -50%", 'kind = "tech_cost_mult"\nresearched_at = [104]\nby_age = { Dark = 0.5 }')],
 "Byzantines": [("Camel Riders, Skirmishers and Spearman-line", 'kind = "unit_cost_mult"\nunits = [329, 330, 207, 7, 6, 93, 358, 359]\nby_age = { Dark = 0.75 }'),
                ("Town Watch, Town Patrol free", 'kind = "tech_free"\ntechs = [8, 280]')],
 "Danes": [("Barracks and Siege Workshop upgrades", 'kind = "tech_cost_mult"\nresearched_at = [12, 49]\nresources = ["gold"]\nby_age = { Dark = 0.34 }')],
 "Dravidians": [("Barracks technologies cost -50%", 'kind = "tech_cost_mult"\nresearched_at = [12]\nby_age = { Dark = 0.5 }'),
                ("Siege Weapons cost -33% wood", 'kind = "unit_cost_mult"\ntrained_at = [49]\nresources = ["wood"]\nby_age = { Dark = 0.67 }')],
 "Ethiopians": [("Pikeman upgrade free", 'kind = "tech_free"\ntechs = [197]')],
 "Goths": [("Infantry costs", 'kind = "unit_cost_mult"\nclasses = [1]\nby_age = { Dark = 0.85, Feudal = 0.80, Castle = 0.75, Imperial = 0.70 }')],
 "Huns": [("Cavalry Archers cost", 'kind = "unit_cost_mult"\nunits = [39, 474]\nby_age = { Castle = 0.9, Imperial = 0.8 }')],
 "Incas": [("Military Units cost", 'kind = "unit_cost_mult"\ntrained_at = [12, 87, 101, 49, 82]\nresources = ["food"]\nby_age = { Dark = 0.85, Feudal = 0.80, Castle = 0.75, Imperial = 0.70 }')],
 "Italians": [("Gunpowder Units cost -20%", 'kind = "unit_cost_mult"\nclasses = [23]\nby_age = { Dark = 0.8 }'),
              ("Dock and University technologies", 'kind = "tech_cost_mult"\nresearched_at = [45, 209]\nby_age = { Dark = 0.75 }'),
              ("Fishing Ships cost -15%", 'kind = "unit_cost_mult"\nunits = [13]\nby_age = { Dark = 0.85 }')],
 "Khitans": [("Heavy Cavalry Archer upgrade", f'kind = "tech_cost_mult"\ntechs = [218]\nby_age = {{ Dark = 0.5 }}\n{A}  # "disponible en Castillos" no se simula')],
 "Koreans": [("Ranged Soldiers and Infantry cost -50% wood", f'kind = "unit_cost_mult"\nclasses = [1, 15]\nresources = ["wood"]\nby_age = {{ Dark = 0.5 }}\n{A}  # "soldados a distancia" ≈ clase arquero'),
             ("Archer armor and tower upgrades free", 'kind = "tech_free"\ntechs = [211, 212, 219]')],
 "Magyars": [("Scout Cavalry-line costs -15%", 'kind = "unit_cost_mult"\nunits = [448, 546, 441]\nby_age = { Dark = 0.85 }'),
             ("Melee attack upgrades free", 'kind = "tech_free"\ntechs = [67, 68, 75]')],
 "Malay": [("Infantry armor upgrades free", 'kind = "tech_free"\ntechs = [74, 76, 77]'),
           ("Battle Elephants cost", 'kind = "unit_cost_mult"\nunits = [1132, 1134]\nby_age = { Castle = 0.75, Imperial = 0.65 }')],
 "Poles": [("Bloodlines and Scout Cavalry-line upgrades", 'kind = "tech_cost_mult"\ntechs = [435, 254, 428]\nresources = ["food"]\nby_age = { Dark = 0.5 }')],
 "Romans": [("Scorpions cost -50% gold", 'kind = "unit_cost_mult"\nunits = [279, 542]\nresources = ["gold"]\nby_age = { Dark = 0.5 }')],
 "Shu": [("Archery Unit technologies", 'kind = "tech_cost_mult"\nresearched_at = [87]\nby_age = { Dark = 0.75 }'),
         ("Archery Unit technologies", 'kind = "tech_cost_mult"\ntechs = [199, 200, 201, 211, 212, 219]\nby_age = { Dark = 0.75 }')],
 "Slavs": [("Siege Workshop Units cost -15%", 'kind = "unit_cost_mult"\ntrained_at = [49]\nby_age = { Dark = 0.85 }'),
           ("Arson, Gambesons free", 'kind = "tech_free"\ntechs = [602, 875]')],
 "Spanish": [("Blacksmith upgrades cost no gold", 'kind = "tech_cost_mult"\nresearched_at = [103]\nresources = ["gold"]\nby_age = { Dark = 0.0 }')],
 "Tatars": [("Thumb Ring, Parthian Tactics free", 'kind = "tech_free"\ntechs = [437, 436]')],
 "Teutons": [("Murder Holes, Herbal Medicine free", 'kind = "tech_free"\ntechs = [322, 441]')],
 "Tupi": [("Archery Range and Barracks upgrades", 'kind = "tech_cost_mult"\nresearched_at = [87, 12]\nresources = ["food"]\nby_age = { Dark = 0.5 }')],
 "Turks": [("Scout Cavalry-line +1 pierce armor and upgrades free", 'kind = "tech_free"\ntechs = [254, 428]'),
           ("Chemistry free", 'kind = "tech_free"\ntechs = [47]')],
 "Vietnamese": [("Conscription free", 'kind = "tech_free"\ntechs = [315]')],
 "Jurchens": [("Siege and Fortification upgrades", f'kind = "tech_cost_mult"\nresearched_at = [49]\nresources = ["wood"]\nby_age = {{ Dark = 0.25 }}\n{A}  # solo las del taller de asedio')],
}
for civ, items in MORE.items():
    S.setdefault(civ, []).extend(items)

out = ['''# Bonos de civilización que el simulador aplica (generado por
# tools/gen_bonuses.py a partir de data/civs.json; editar el script, no este archivo).
# Cada bono cita la línea oficial del juego (en español) que lo respalda; los
# bonos que no aparecen aquí (militares, navales, de mapa, sin cifras) se
# muestran en la app como "no simulados".
#
# kind: no_houses, start_resources_add, start_villagers_add, rate_mult, carry_add,
#   source_amount_mult, villager_speed_mult, building_cost_mult, building_cost_add,
#   tech_free, tech_cost_mult, unit_cost_mult, production_speed_mult,
#   research_speed_mult, research_reward, tech_villagers, build_speed_mult,
#   age_reward, age_villagers, building_reward, population_add, farm_food_mult,
#   farm_tech_mult, no_age_buildings (ver crates/aoe2-data/src/civs.rs)
schema_version = 2
''']
ids = {c['id'] for c in d}
for civ, items in sorted(S.items()):
    assert civ in ids, civ
    c = next(x for x in d if x['id'] == civ)
    en, te = bullets(c['description']['en'])
    es, ts = bullets(c['description']['es'])
    out.append(f'\n[[civ]]\nid = "{civ}"\n')
    for key, body in items:
        if key == "TEAM":
            line_en, line_es = te, ts
        else:
            hits = [i for i, l in enumerate(en) if key in l]
            assert len(hits) == 1, (civ, key, en)
            line_en, line_es = en[hits[0]], es[hits[0]]
        line_es = line_es.replace('"', '\\"')
        out.append(f'\n[[civ.bonus]]\n{body}\ntext = "{line_es}"\n# {line_en}\n')
open(f'{root}/data/civ_bonuses.toml', 'w').write(''.join(out))
print(len(S), 'civilizaciones con bonos simulados')


# --- Bonos militares (pestaña Batalla) -> data/combat_civ_bonuses.toml ---
MOUNTED = '{ categories = ["cavalry", "cav_archers"] }'
B = {
 "Franks": [("Mounted Units +20% HP", MOUNTED, '{ Feudal = { hp_mult = 1.2 } }')],
 "Britons": [("Foot Archers +1/+2 range", '{ categories = ["archers"] }', '{ Castle = { range = 1 }, Imperial = { range = 2 } }')],
 "Burmese": [("Infantry +1/+2/+3 attack", '{ categories = ["infantry"] }', '{ Feudal = { melee_attack = 1 }, Castle = { melee_attack = 2 }, Imperial = { melee_attack = 3 } }'),
             ("Battle Elephants +1 melee", '{ units = [1132, 1134] }', '{ Dark = { melee_armor = 1, pierce_armor = 1 } }')],
 "Teutons": [("Infantry and Mounted Units +1/+2 melee armor", '{ categories = ["infantry", "cavalry", "cav_archers"] }', '{ Castle = { melee_armor = 1 }, Imperial = { melee_armor = 2 } }')],
 "Vikings": [("Infantry +20% HP", '{ categories = ["infantry"] }', '{ Feudal = { hp_mult = 1.2 } }')],
 "Japanese": [("Infantry attacks +33% faster", '{ categories = ["infantry"] }', '{ Feudal = { reload_mult = 1.33 } }')],
 "Mongols": [("Cavalry Archers attack +25% faster", '{ categories = ["cav_archers"] }', '{ Dark = { reload_mult = 1.25 } }'),
             ("Scout Cavalry-line and Steppe Lancers", '{ units = [448, 546, 441] }', '{ Castle = { hp_mult = 1.2 }, Imperial = { hp_mult = 1.3 } }')],
 "Malians": [("Barracks Units +1/+2/+3 pierce armor", '{ trained_at = [12] }', '{ Feudal = { pierce_armor = 1 }, Castle = { pierce_armor = 2 }, Imperial = { pierce_armor = 3 } }')],
 "Italians": [("Foot Archers and Condottieri", '{ categories = ["archers"] }', '{ Dark = { melee_armor = 1, pierce_armor = 1 } }')],
 "Ethiopians": [("Foot Archers attack +18% faster", '{ categories = ["archers"] }', '{ Dark = { reload_mult = 1.18 } }')],
 "Saracens": [("Camel Units +25% HP", '{ classes = [30] }', '{ Dark = { hp_mult = 1.25 } }')],
 "Turks": [("Gunpowder Units +25% HP", '{ classes = [23] }', '{ Dark = { hp_mult = 1.25 } }'),
           ("Scout Cavalry-line +1 pierce armor", '{ units = [448, 546, 441] }', '{ Dark = { pierce_armor = 1 } }')],
 "Hindustanis": [("Camel Riders attack +20% faster", '{ units = [329, 330, 207] }', '{ Dark = { reload_mult = 1.2 } }'),
                 ("Gunpowder Units +1 melee", '{ classes = [23] }', '{ Dark = { melee_armor = 1, pierce_armor = 1 } }')],
 "Celts": [("Infantry moves", '{ categories = ["infantry"] }', '{ Dark = { speed_mult = 1.05 }, Feudal = { speed_mult = 1.10 }, Castle = { speed_mult = 1.15 }, Imperial = { speed_mult = 1.20 } }')],
 "Lithuanians": [("Spearman-line and Skirmisher-line move", '{ units = [93, 358, 359, 7, 6] }', '{ Dark = { speed_mult = 1.10 } }')],
 "Cumans": [("Mounted Units move", MOUNTED, '{ Feudal = { speed_mult = 1.05 }, Castle = { speed_mult = 1.10 }, Imperial = { speed_mult = 1.15 } }')],
 "Mapuche": [("Infantry, Slingers and Skirmishers", '{ categories = ["infantry"], units = [7, 6] }', '{ Feudal = { hp = 5 }, Castle = { hp = 10 }, Imperial = { hp = 15 } }')],
 "Muisca": [("Archery Range Units +1/2/3 melee armor", '{ trained_at = [87] }', '{ Feudal = { melee_armor = 1 }, Castle = { melee_armor = 2 }, Imperial = { melee_armor = 3 } }')],
}
HEAD = """# Bonos militares de civilización para la pestaña Batalla (generado por
# tools/gen_bonuses.py; editar el script). Mismo formato de efectos que
# data/combat_upgrades.toml, por edad (el de la última edad alcanzada).
schema_version = 1
"""
bout = [HEAD]
for civ, items in sorted(B.items()):
    c = next(x for x in d if x['id'] == civ)
    en, te = bullets(c['description']['en'])
    es, ts = bullets(c['description']['es'])
    for key, applies, by_age in items:
        hits = [i for i, l in enumerate(en) if key in l]
        assert len(hits) == 1, (civ, key, en)
        line = es[hits[0]].replace('"', '\\"')
        bout.append(f'\n[[civ]]\nid = "{civ}"\napplies = {applies}\nby_age = {by_age}\ntext = "{line}"\n# {en[hits[0]]}\n')
open(f'{root}/data/combat_civ_bonuses.toml', 'w').write(''.join(bout))
print(len(B), 'civilizaciones con bonos militares')
