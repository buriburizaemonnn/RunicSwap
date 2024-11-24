import { useState } from "react";
import { TokenType } from "./TokenSelector";
import { TokenSelector } from "./TokenSelector";
import "./../styles/TokenSelector.css";

interface DualTokenSelectorProps {
  onSelectFirst: (token: TokenType) => void;
  onSelectSecond: (token: TokenType) => void;
}

export const DualTokenSelector: React.FC<DualTokenSelectorProps> = ({
  onSelectFirst,
  onSelectSecond,
}) => {
  const [firstToken, setFirstToken] = useState<TokenType | undefined>();
  const [secondToken, setSecondToken] = useState<TokenType | undefined>();

  const handleFirstSelect = (token: TokenType) => {
    setFirstToken(token);
    onSelectFirst(token);
  };

  const handleSecondSelect = (token: TokenType) => {
    setSecondToken(token);
    onSelectSecond(token);
  };

  return (
    <div className="token-selector-container">
      <TokenSelector
        onSelect={handleFirstSelect}
        otherSelectedToken={secondToken}
      />
      <TokenSelector
        onSelect={handleSecondSelect}
        otherSelectedToken={firstToken}
      />
    </div>
  );
};
