import React from "react";

interface BadgeProps {
  children: React.ReactNode;
  variant?: "primary" | "success" | "secondary";
  className?: string;
}

const Badge: React.FC<BadgeProps> = ({
  children,
  variant = "primary",
  className = "",
}) => {
  const variantClasses = {
    // `text-on-logo-primary` ist Pflicht, nicht Kosmetik: die Fläche ist im
    // Hellmodus dunkel und im Dunkelmodus hell. Ohne eigene Textfarbe erbt der
    // Text den normalen Textton — und der ist im Dunkelmodus genauso hell wie
    // die Fläche, das Badge wird unlesbar.
    primary: "bg-logo-primary text-on-logo-primary",
    success: "bg-mid-gray/20 text-text/70",
    secondary: "bg-mid-gray/20 text-text/70",
  };

  return (
    <span
      className={`inline-flex items-center px-3 py-1 rounded-full text-xs font-medium ${variantClasses[variant]} ${className}`}
    >
      {children}
    </span>
  );
};

export default Badge;
