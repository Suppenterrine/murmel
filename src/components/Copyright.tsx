/* eslint-disable i18next/no-literal-string -- Rechtevermerk mit Eigennamen; wird bewusst nicht übersetzt. */

/**
 * Rechtevermerk unter der Wortmarke. Eine eigene Komponente, damit Startbild
 * und Info-Seite denselben Text zeigen und er nur an einer Stelle steht.
 */
export const Copyright = ({ className = "" }: { className?: string }) => (
  <p className={`text-xs text-text/50 tracking-wide ${className}`}>
    © {new Date().getFullYear()} Lukas Baumert
  </p>
);

export default Copyright;
