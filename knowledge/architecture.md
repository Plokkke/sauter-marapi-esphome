# Architecture et contrat d'interface

## Rôles

| Niveau | Projet | Rôle |
|---|---|---|
| Sèche-serviettes | Sauter Marapi tactile 1750 W (750 W inertie fluide + 1000 W soufflerie) | thermostat électronique intégré ; en Confort, il régule sur la consigne affichée |
| Pont | **ce projet** | traduit un ordre choisi dans HA en signal fil pilote 6 ordres |
| Maison | automatisation HA (autre projet) | plannings, présence, délestage |

Ce projet ne contient aucune automatisation HA. La liaison est **unidirectionnelle** : le fil pilote ne renvoie ni température, ni état de chauffe, ni défaut.

## Entité exposée

| Entité | Type | Valeurs |
|---|---|---|
| Pilot wire order | `select` | `Comfort`, `Comfort -1`, `Comfort -2`, `Eco`, `Frost protection`, `Off` |

L'ordre est restauré après une coupure de courant (`restore_value`). Les températures Confort et Éco se règlent sur l'écran de l'appareil (Éco = Confort − 3,5 °C par défaut, réglable de −1 à −9 °C dans « Abais. T ECO », notice p. 15).

## Sécurité (firmware)

- **DÉCIDÉ : repli en Éco**, jamais en Confort (qui chaufferait la salle de bains en continu).
- Matériel : relais à contact repos ; carte hors tension, en panne ou en démarrage → 230 V sur le fil pilote → Éco.
- Logiciel : si HA est injoignable pendant **30 min** (depuis le démarrage ou la dernière déconnexion), la carte repasse en `Eco`. L'ordre par défaut, sans valeur restaurée, est aussi `Eco`. `api.reboot_timeout` est désactivé pour éviter un redémarrage en boucle.

## Alimentation

- **DÉCIDÉ** : module alimenté par l'arrivée murale du sèche-serviettes (phase + neutre, en aval du différentiel 30 mA de la salle de bains), via un HLK-PM01 protégé par un fusible.
- Le module s'intercale entre la boîte de raccordement murale et le câble de l'appareil : phase et neutre traversent, le fil noir sort du module.
