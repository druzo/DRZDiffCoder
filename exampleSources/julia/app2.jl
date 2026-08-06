# Julia — multiple dispatch + a small geometry example.

abstract type Shape end

struct Circle   <: Shape; radius::Float64; end
struct Rectangle <: Shape; w::Float64; h::Float64; end

area(c::Circle)    = pi * c.radius^2
area(r::Rectangle) = r.w * r.h

describe(s::Shape) = "$(typeof(s).name.name): area = $(round(area(s); digits=2))"

function main()
    shapes = Shape[
        Circle(2.0),
        Rectangle(3.0, 4.0),
        Circle(1.5),
    ]
    total = sum(area, shapes)
    for s in shapes
        println(describe(s))
    end
    println("total = ", round(total; digits=2))
end

main()