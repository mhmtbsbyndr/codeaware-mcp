package com.example;

public class UserService {
    private String name;

    public UserService(String name) {
        this.name = name;
    }

    public String getName() {
        return this.name;
    }

    private void validate(String input) {
        if (input == null) {
            throw new IllegalArgumentException("null input");
        }
    }
}

interface Repository {
    void save(Object entity);
    Object findById(int id);
}

enum Status {
    ACTIVE,
    INACTIVE,
    DELETED
}
