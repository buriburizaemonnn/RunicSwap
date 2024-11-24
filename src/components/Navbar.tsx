import React from "react";
import { Link } from "react-router-dom";

const Navbar: React.FC = () => {
  return (
    <nav style={styles.navbar}>
      <div style={styles.logo}>RunicSwap</div>
      <div style={styles.navLinks}>
        <Link to="/" style={styles.link}>
          Swap
        </Link>
        <Link to="/pools" style={styles.link}>
          Pools
        </Link>
        <Link to="/positions" style={styles.link}>
          My Positions
        </Link>
      </div>
      <button>Connect Wallet</button>
    </nav>
  );
};

const styles: Record<string, React.CSSProperties> = {
  navbar: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    padding: "1rem 2rem",
    backgroundColor: "#1a1a1a",
    color: "white",
    position: "fixed",
    top: 0,
    left: 0,
    right: 0,
    boxShadow: "0 2px 5px rgba(0,0,0,0.2)",
  },
  logo: {
    fontSize: "1.5rem",
    fontWeight: "bold",
    color: "#00ff9d",
    cursor: "pointer",
  },
  navLinks: {
    display: "flex",
    gap: "2rem",
  },
  link: {
    color: "white",
    textDecoration: "none",
    fontSize: "1rem",
    padding: "0.5rem",
    transition: "color 0.3s ease",
  },
};

export default Navbar;
