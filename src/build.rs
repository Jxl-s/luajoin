use full_moon::{
    tokenizer::{Token, TokenReference, TokenType},
    visitors::VisitorMut,
};

pub struct BuildVisitor;

impl VisitorMut for BuildVisitor {
    fn visit_token_reference(
        &mut self,
        node: full_moon::tokenizer::TokenReference,
    ) -> full_moon::tokenizer::TokenReference {
        let leading_trivia: Vec<Token> = Vec::from_iter(node.leading_trivia().map(|token| {
            let token = token.clone();

            if let TokenType::Whitespace { characters } = token.token_type() {
                if characters.contains("\n") || characters.contains("\r") {
                    return Token::new(TokenType::Whitespace {
                        characters: " ".into(),
                    });
                } else if !characters.is_empty() {
                    return Token::new(TokenType::Whitespace {
                        characters: "".into(),
                    });
                }

                return token;
            }

            return token;
        }));

        let trailing_trivia: Vec<Token> = Vec::from_iter(node.trailing_trivia().map(|token| {
            let token = token.clone();

            if let TokenType::Whitespace { characters } = token.token_type() {
                if characters.contains("\n") || characters.contains("\r") {
                    return Token::new(TokenType::Whitespace {
                        characters: " ".into(),
                    });
                } else if !characters.is_empty() {
                    return Token::new(TokenType::Whitespace {
                        characters: " ".into(),
                    });
                }

                return token;
            }

            return token;
        }));

        TokenReference::new(leading_trivia, node.token().clone(), trailing_trivia)
    }

    // Removing comments
    fn visit_single_line_comment(
        &mut self,
        _: full_moon::tokenizer::Token,
    ) -> full_moon::tokenizer::Token {
        Token::new(TokenType::Whitespace {
            characters: "".into(),
        })
    }

    fn visit_multi_line_comment(
        &mut self,
        _: full_moon::tokenizer::Token,
    ) -> full_moon::tokenizer::Token {
        Token::new(TokenType::Whitespace {
            characters: "".into(),
        })
    }
}
