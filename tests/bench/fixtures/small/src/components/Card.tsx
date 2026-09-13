import React from 'react';

interface CardProps {
  title: string;
  children: React.ReactNode;
}

export function Card({ title, children }: CardProps) {
  return (
    <div className="card" role="region" aria-labelledby="card-title">
      <h2 id="card-title">{title}</h2>
      <div className="card-body">{children}</div>
    </div>
  );
}
