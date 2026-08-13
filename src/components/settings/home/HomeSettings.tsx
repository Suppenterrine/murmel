import React from "react";
import { Trans, useTranslation } from "react-i18next";
import MurmelHand from "../../icons/MurmelHand";

/**
 * Startseite — die erste Ansicht beim Öffnen des Fensters.
 *
 * Bewusst ohne Bedienelemente. Murmel wird über Tastenkürzel benutzt, nicht
 * über dieses Fenster; wer es öffnet, will meist etwas nachschlagen oder
 * einstellen. Ein ruhiger Einstieg ist hier mehr wert als eine weitere Wand aus
 * Schaltern — die stehen einen Klick weiter.
 *
 * Gesetzt in Fraunces statt in der UI-Schrift: Dies ist Text zum Lesen, kein
 * Bedienelement, und die Wortmarke gibt den Ton bereits vor.
 */
export const HomeSettings: React.FC = () => {
  const { t } = useTranslation();

  return (
    <div className="max-w-2xl w-full mx-auto">
      <div className="flex flex-col items-center gap-6 py-10">
        <MurmelHand width={128} height={128} />

        <h1 className="prose-display text-4xl font-bold tracking-tight text-text">
          {t("home.headline")}
        </h1>

        <p className="prose-display text-lg leading-relaxed text-center text-text/80 text-balance">
          {t("home.lead")}
        </p>
      </div>

      <div className="prose-display space-y-5 text-base leading-relaxed text-text/75">
        <p>{t("home.how")}</p>
        <p>
          <Trans
            i18nKey="home.privacy"
            components={{ strong: <strong className="text-text" /> }}
          />
        </p>
        <p>{t("home.refinement")}</p>
      </div>

      <p className="prose-display mt-10 text-center text-sm italic text-text/50">
        {t("home.motto")}
      </p>
    </div>
  );
};
